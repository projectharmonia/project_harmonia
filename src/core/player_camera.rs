mod exp_smoothed;

use std::f32::consts::FRAC_PI_2;

use bevy::{
    ecs::system::SystemParam, input::mouse::MouseMotion, pbr::ScreenSpaceAmbientOcclusionBundle,
    prelude::*, window::PrimaryWindow,
};
use leafwing_input_manager::prelude::ActionState;

use self::exp_smoothed::ExpSmoothed;

use super::{action::Action, game_state::GameState};

pub(super) struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                (
                    Self::update_rotation,
                    Self::update_origin.run_if(not(in_state(GameState::FamilyEditor))),
                    Self::update_spring_arm,
                ),
                Self::apply_transform,
            )
                .chain()
                .run_if(
                    in_state(GameState::FamilyEditor)
                        .or_else(in_state(GameState::City))
                        .or_else(in_state(GameState::Family)),
                ),
        );
    }
}

impl PlayerCameraPlugin {
    fn update_rotation(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut motion_events: EventReader<MouseMotion>,
        mut cameras: Query<&mut OrbitRotation, With<PlayerCamera>>,
    ) {
        let mut orbit_rotation = cameras.single_mut();
        let motion = motion_events.read().map(|event| &event.delta).sum::<Vec2>();
        if action_state.pressed(&Action::RotateCamera) {
            const SENSETIVITY: f32 = 0.01;
            orbit_rotation.dest -= SENSETIVITY * motion;
            orbit_rotation.dest.y = orbit_rotation.dest.y.clamp(0.0, FRAC_PI_2);
        }
        orbit_rotation.smooth(time.delta_seconds());
    }

    fn update_origin(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut cameras: Query<(&mut OrbitOrigin, &Transform), With<PlayerCamera>>,
    ) {
        let (mut orbit_origin, transform) = cameras.single_mut();

        const MOVEMENT_SPEED: f32 = 10.0;
        orbit_origin.dest += movement_direction(&action_state, transform.rotation)
            * time.delta_seconds()
            * MOVEMENT_SPEED;
        orbit_origin.smooth(time.delta_seconds());
    }

    fn update_spring_arm(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut cameras: Query<&mut SpringArm, With<PlayerCamera>>,
    ) {
        let mut spring_arm = cameras.single_mut();
        spring_arm.dest = (spring_arm.dest - action_state.value(&Action::ZoomCamera)).max(0.0);
        spring_arm.smooth(time.delta_seconds());
    }

    fn apply_transform(
        mut cameras: Query<
            (&mut Transform, &OrbitOrigin, &OrbitRotation, &SpringArm),
            With<PlayerCamera>,
        >,
    ) {
        let (mut transform, orbit_origin, orbit_rotation, spring_arm) = cameras.single_mut();
        transform.translation =
            orbit_rotation.sphere_pos() * spring_arm.value() + orbit_origin.value();
        transform.look_at(orbit_origin.value(), Vec3::Y);
    }
}

fn movement_direction(action_state: &ActionState<Action>, rotation: Quat) -> Vec3 {
    let mut direction = Vec3::ZERO;
    if action_state.pressed(&Action::CameraLeft) {
        direction.x -= 1.0;
    }
    if action_state.pressed(&Action::CameraRight) {
        direction.x += 1.0;
    }
    if action_state.pressed(&Action::CameraForward) {
        direction.z -= 1.0;
    }
    if action_state.pressed(&Action::CameraBackward) {
        direction.z += 1.0;
    }

    direction = rotation * direction;
    direction.y = 0.0;

    direction.normalize_or_zero()
}

#[derive(Bundle)]
pub(crate) struct PlayerCameraBundle {
    orbit_origin: OrbitOrigin,
    orbit_rotation: OrbitRotation,
    spring_arm: SpringArm,
    player_camera: PlayerCamera,
    camera_3d_bundle: Camera3dBundle,
    ssao_bundle: ScreenSpaceAmbientOcclusionBundle,
}

impl Default for PlayerCameraBundle {
    fn default() -> Self {
        Self {
            orbit_origin: Default::default(),
            orbit_rotation: Default::default(),
            spring_arm: Default::default(),
            player_camera: PlayerCamera,
            camera_3d_bundle: Camera3dBundle {
                camera: Camera {
                    hdr: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            ssao_bundle: Default::default(),
        }
    }
}

/// The origin of a camera.
#[derive(Component, Default, Deref, DerefMut)]
struct OrbitOrigin(ExpSmoothed<Vec3>);

/// Camera rotation in `X` and `Z`.
#[derive(Component, Deref, DerefMut)]
struct OrbitRotation(ExpSmoothed<Vec2>);

impl OrbitRotation {
    fn sphere_pos(&self) -> Vec3 {
        Quat::from_euler(EulerRot::YXZ, self.value().x, self.value().y, 0.0) * Vec3::Y
    }
}

impl Default for OrbitRotation {
    fn default() -> Self {
        Self(ExpSmoothed::new(Vec2::new(0.0, 60_f32.to_radians())))
    }
}

/// Camera distance.
#[derive(Component, Deref, DerefMut)]
struct SpringArm(ExpSmoothed<f32>);

impl Default for SpringArm {
    fn default() -> Self {
        Self(ExpSmoothed::new(10.0))
    }
}

#[derive(Component, Default)]
pub(super) struct PlayerCamera;

/// A helper to cast rays from [`PlayerCamera`].
#[derive(SystemParam)]
pub(super) struct CameraCaster<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    pub(super) cameras:
        Query<'w, 's, (&'static GlobalTransform, &'static Camera), With<PlayerCamera>>,
}

impl CameraCaster<'_, '_> {
    pub(super) fn ray(&self) -> Option<Ray3d> {
        let cursor_pos = self.windows.single().cursor_position()?;
        let (&transform, camera) = self.cameras.get_single().ok()?;
        camera.viewport_to_world(&transform, cursor_pos)
    }

    pub(super) fn intersect_ground(&self) -> Option<Vec3> {
        let ray = self.ray()?;
        let distance = ray.intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y))?;
        Some(ray.get_point(distance))
    }
}
