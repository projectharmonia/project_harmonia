mod exp_smoothed;

use std::f32::consts::FRAC_PI_2;

use bevy::{input::mouse::MouseMotion, prelude::*};
use leafwing_input_manager::prelude::ActionState;

use self::exp_smoothed::ExpSmoothed;

use super::{action::Action, game_state::GameState};

pub(super) struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            PlayerCameraSet.run_if(
                in_state(GameState::FamilyEditor)
                    .or_else(in_state(GameState::City))
                    .or_else(in_state(GameState::Family)),
            ),
        )
        .add_system(
            Self::position_system
                .run_if(in_state(GameState::City).or_else(in_state(GameState::Family))),
        )
        .add_systems(
            (
                Self::rotation_system,
                Self::arm_system,
                Self::transform_system
                    .after(Self::position_system)
                    .after(Self::rotation_system)
                    .after(Self::arm_system),
            )
                .in_set(PlayerCameraSet),
        );
    }
}

impl PlayerCameraPlugin {
    fn rotation_system(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut motion_events: EventReader<MouseMotion>,
        mut cameras: Query<&mut OrbitRotation, With<PlayerCamera>>,
    ) {
        let mut orbit_rotation = cameras.single_mut();
        if action_state.pressed(Action::RotateCamera) {
            const SENSETIVITY: f32 = 0.01;
            orbit_rotation.dest -=
                SENSETIVITY * motion_events.iter().map(|event| &event.delta).sum::<Vec2>();
            orbit_rotation.dest.y = orbit_rotation.dest.y.clamp(0.0, FRAC_PI_2);
        }
        orbit_rotation.smooth(time.delta_seconds());
    }

    fn position_system(
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

    fn arm_system(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut cameras: Query<&mut SpringArm, With<PlayerCamera>>,
    ) {
        let mut spring_arm = cameras.single_mut();
        spring_arm.dest = (spring_arm.dest - action_state.value(Action::ZoomCamera)).max(0.0);
        spring_arm.smooth(time.delta_seconds());
    }

    fn transform_system(
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
    if action_state.pressed(Action::CameraLeft) {
        direction.x -= 1.0;
    }
    if action_state.pressed(Action::CameraRight) {
        direction.x += 1.0;
    }
    if action_state.pressed(Action::CameraForward) {
        direction.z -= 1.0;
    }
    if action_state.pressed(Action::CameraBackward) {
        direction.z += 1.0;
    }

    direction = rotation * direction;
    direction.y = 0.0;

    direction.normalize_or_zero()
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct PlayerCameraSet;

#[derive(Bundle, Default)]
pub(crate) struct PlayerCameraBundle {
    target_translation: OrbitOrigin,
    orbit_rotation: OrbitRotation,
    spring_arm: SpringArm,
    player_camera: PlayerCamera,

    #[bundle]
    camera_3d_bundle: Camera3dBundle,
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
