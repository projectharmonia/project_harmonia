use std::f32::consts::FRAC_PI_2;

use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_mod_raycast::{RaycastMethod, RaycastSource};
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{
    action::{self, Action},
    game_state::GameState,
    picking::Pickable,
};

#[derive(SystemLabel)]
enum OrbitCameraSystem {
    Rotation,
    Position,
    Arm,
}

pub(super) struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        for state in [GameState::FamilyEditor, GameState::City, GameState::Family] {
            app.add_system(Self::update_raycast_source_system.run_in_state(state))
                .add_system(
                    Self::rotation_system
                        .run_if(action::pressed(Action::RotateCamera))
                        .run_in_state(state)
                        .label(OrbitCameraSystem::Rotation),
                )
                .add_system(
                    Self::arm_system
                        .run_in_state(state)
                        .label(OrbitCameraSystem::Arm),
                );

            if state != GameState::FamilyEditor {
                app.add_system(
                    Self::position_system
                        .run_in_state(state)
                        .label(OrbitCameraSystem::Position),
                )
                .add_system(
                    Self::transform_system
                        .run_in_state(state)
                        .after(OrbitCameraSystem::Rotation)
                        .after(OrbitCameraSystem::Arm)
                        .after(OrbitCameraSystem::Position),
                );
            } else {
                app.add_system(
                    Self::transform_system
                        .run_in_state(state)
                        .after(OrbitCameraSystem::Rotation)
                        .after(OrbitCameraSystem::Arm),
                );
            }
        }
    }
}

impl OrbitCameraPlugin {
    /// Interpolation multiplier for movement and camera zoom.
    const INTERPOLATION_SPEED: f32 = 5.0;

    fn rotation_system(
        mut motion_events: EventReader<MouseMotion>,
        mut camera: Query<&mut OrbitRotation, With<Camera>>,
    ) {
        let mut orbit_rotation = camera.single_mut();
        const SENSETIVITY: f32 = 0.01;
        orbit_rotation.0 -=
            SENSETIVITY * motion_events.iter().map(|event| &event.delta).sum::<Vec2>();
        orbit_rotation.y = orbit_rotation.y.clamp(0.0, FRAC_PI_2);
    }

    fn position_system(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut camera: Query<(&mut OrbitOrigin, &Transform), With<Camera>>,
    ) {
        let (mut orbit_origin, transform) = camera.single_mut();

        const MOVEMENT_SPEED: f32 = 10.0;
        orbit_origin.current += movement_direction(&action_state, transform.rotation)
            * time.delta_seconds()
            * MOVEMENT_SPEED;

        orbit_origin.interpolated = orbit_origin.interpolated.lerp(
            orbit_origin.current,
            time.delta_seconds() * Self::INTERPOLATION_SPEED,
        );
    }

    fn arm_system(
        time: Res<Time>,
        action_state: Res<ActionState<Action>>,
        mut camera: Query<&mut SpringArm, With<Camera>>,
    ) {
        let mut spring_arm = camera.single_mut();
        spring_arm.current = (spring_arm.current - action_state.value(Action::ZoomCamera)).max(0.0);
        spring_arm.interpolated = spring_arm.interpolated
            + time.delta_seconds()
                * Self::INTERPOLATION_SPEED
                * (spring_arm.current - spring_arm.interpolated);
    }

    fn transform_system(
        mut camera: Query<(&mut Transform, &OrbitOrigin, &OrbitRotation, &SpringArm), With<Camera>>,
    ) {
        let (mut transform, orbit_origin, orbit_rotation, spring_arm) = camera.single_mut();
        transform.translation =
            orbit_rotation.sphere_pos() * spring_arm.interpolated + orbit_origin.interpolated;
        transform.look_at(orbit_origin.interpolated, Vec3::Y);
    }

    fn update_raycast_source_system(
        mut cursor_events: EventReader<CursorMoved>,
        mut ray_sources: Query<&mut RaycastSource<Pickable>>,
    ) {
        if let Some(cursor_pos) = cursor_events.iter().last().map(|event| event.position) {
            for mut ray_source in &mut ray_sources {
                ray_source.cast_method = RaycastMethod::Screenspace(cursor_pos);
            }
        }
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

#[derive(Bundle, Default)]
pub(crate) struct OrbitCameraBundle {
    target_translation: OrbitOrigin,
    orbit_rotation: OrbitRotation,
    spring_arm: SpringArm,
    ray_source: RaycastSource<Pickable>,

    #[bundle]
    camera_3d: Camera3dBundle,
}

/// The origin of a camera.
#[derive(Component, Default)]
pub(super) struct OrbitOrigin {
    current: Vec3,
    interpolated: Vec3,
}

/// Camera rotation in `X` and `Z`.
#[derive(Component, Deref, DerefMut, Clone, Copy)]
struct OrbitRotation(Vec2);

impl OrbitRotation {
    fn sphere_pos(self) -> Vec3 {
        Quat::from_euler(EulerRot::YXZ, self.x, self.y, 0.0) * Vec3::Y
    }
}

impl Default for OrbitRotation {
    fn default() -> Self {
        Self(Vec2::new(0.0, 60_f32.to_radians()))
    }
}

/// Camera distance.
#[derive(Component)]
struct SpringArm {
    current: f32,
    interpolated: f32,
}

impl Default for SpringArm {
    fn default() -> Self {
        Self {
            current: 10.0,
            interpolated: 0.0,
        }
    }
}
