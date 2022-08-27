use std::f32::consts::FRAC_PI_2;

use bevy::{input::mouse::MouseMotion, prelude::*};
use bevy_mod_raycast::{RayCastMethod, RayCastSource};
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{city::City, control_action::ControlAction, game_state::GameState, object::ObjectPath};

#[derive(SystemLabel)]
enum OrbitCameraSystem {
    Rotation,
    Position,
    Arm,
}

pub(super) struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::City, Self::spawn_system)
            .add_system(
                Self::rotation_system
                    .run_in_state(GameState::City)
                    .run_if(is_rotating_camera)
                    .label(OrbitCameraSystem::Rotation),
            )
            .add_system(
                Self::position_system
                    .run_in_state(GameState::City)
                    .label(OrbitCameraSystem::Position),
            )
            .add_system(
                Self::arm_system
                    .run_in_state(GameState::City)
                    .label(OrbitCameraSystem::Arm),
            )
            .add_system(
                Self::transform_system
                    .run_in_state(GameState::City)
                    .after(OrbitCameraSystem::Rotation)
                    .after(OrbitCameraSystem::Position)
                    .after(OrbitCameraSystem::Arm),
            )
            .add_system(Self::update_raycast_source_system.run_in_state(GameState::City));
    }
}

impl OrbitCameraPlugin {
    /// Interpolation multiplier for movement and camera zoom.
    const INTERPOLATION_SPEED: f32 = 5.0;

    fn spawn_system(
        mut commands: Commands,
        controlled_city: Query<(Entity, Option<&Children>), (With<Visibility>, With<City>)>,
        camera: Query<Entity, With<OrbitOrigin>>,
    ) {
        let (city_entity, children) = controlled_city.single();
        let camera_entity = children
            .and_then(|children| camera.iter_many(children).next())
            .unwrap_or_else(|| {
                commands
                    .entity(city_entity)
                    .add_children(|parent| parent.spawn_bundle(OrbitCameraBundle::default()).id())
            });

        commands
            .entity(camera_entity)
            .insert_bundle(Camera3dBundle::default());
    }

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
        action_state: Res<ActionState<ControlAction>>,
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
        action_state: Res<ActionState<ControlAction>>,
        mut camera: Query<&mut SpringArm, With<Camera>>,
    ) {
        let mut spring_arm = camera.single_mut();
        spring_arm.current =
            (spring_arm.current - action_state.value(ControlAction::ZoomCamera)).max(0.0);
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
        mut ray_sources: Query<&mut RayCastSource<ObjectPath>>,
    ) {
        if let Some(cursor_pos) = cursor_events.iter().last().map(|event| event.position) {
            for mut ray_source in &mut ray_sources {
                ray_source.cast_method = RayCastMethod::Screenspace(cursor_pos);
            }
        }
    }
}

fn is_rotating_camera(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.pressed(ControlAction::RotateCamera)
}

fn movement_direction(action_state: &ActionState<ControlAction>, rotation: Quat) -> Vec3 {
    let mut direction = Vec3::ZERO;
    if action_state.pressed(ControlAction::CameraLeft) {
        direction.x -= 1.0;
    }
    if action_state.pressed(ControlAction::CameraRight) {
        direction.x += 1.0;
    }
    if action_state.pressed(ControlAction::CameraForward) {
        direction.z -= 1.0;
    }
    if action_state.pressed(ControlAction::CameraBackward) {
        direction.z += 1.0;
    }

    direction = rotation * direction;
    direction.y = 0.0;

    direction.normalize_or_zero()
}

#[derive(Bundle, Default)]
pub(super) struct OrbitCameraBundle {
    target_translation: OrbitOrigin,
    orbit_rotation: OrbitRotation,
    spring_arg: SpringArm,
    ray_source: RayCastSource<ObjectPath>,
}

/// The origin of a camera.
#[derive(Component, Default)]
struct OrbitOrigin {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{core::CorePlugin, time::TimePlugin, window::WindowId};

    #[test]
    fn movement_direction_normalization() {
        let mut action_state = ActionState::<ControlAction>::default();
        action_state.press(ControlAction::CameraForward);
        action_state.press(ControlAction::CameraRight);

        let direction = movement_direction(&action_state, Quat::IDENTITY);
        assert!(direction.is_normalized());
        assert_eq!(direction.y, 0.0);
    }

    #[test]
    fn movement_direction_compensation() {
        let mut action_state = ActionState::<ControlAction>::default();
        action_state.press(ControlAction::CameraForward);
        action_state.press(ControlAction::CameraBackward);
        action_state.press(ControlAction::CameraRight);
        action_state.press(ControlAction::CameraLeft);

        let direction = movement_direction(&action_state, Quat::IDENTITY);
        assert_eq!(direction.x, 0.0);
        assert_eq!(direction.z, 0.0);
    }

    #[test]
    fn movement_direction_empty() {
        let action_state = ActionState::<ControlAction>::default();

        let direction = movement_direction(&action_state, Quat::IDENTITY);
        assert_eq!(direction, Vec3::ZERO);
    }

    #[test]
    fn spawning_and_controlling() {
        let mut app = App::new();
        app.add_plugin(TestOrbitCameraPlugin);

        let city_entity = app
            .world
            .spawn()
            .insert(City)
            .insert(Visibility::default())
            .id();
        app.world.insert_resource(NextState(GameState::City));

        app.update();

        let (camera_entity, parent) = app
            .world
            .query_filtered::<(Entity, &Parent), With<Camera>>()
            .single(&app.world);

        assert_eq!(
            parent.get(),
            city_entity,
            "Camera should be spawned as a child",
        );

        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::RotateCamera);
        action_state.press(ControlAction::CameraForward);

        app.update();

        let transform = app.world.get::<Transform>(camera_entity).unwrap();
        assert_ne!(transform.translation, Vec3::ZERO);
        assert!(!transform.rotation.is_nan());
    }

    #[test]
    fn ray_source_updating() {
        let mut app = App::new();
        app.add_plugin(TestOrbitCameraPlugin);

        app.world.spawn().insert(City).insert(Visibility::default());
        app.world.insert_resource(NextState(GameState::City));

        app.update();

        const CURSOR_POS: Vec2 = Vec2::ONE;
        let mut cursor_events = app.world.resource_mut::<Events<CursorMoved>>();
        cursor_events.send(CursorMoved {
            id: WindowId::new(),
            position: CURSOR_POS,
        });

        app.world.insert_resource(NextState(GameState::City));

        app.update();

        let ray_source = app
            .world
            .query::<&RayCastSource<ObjectPath>>()
            .single(&app.world);
        assert!(
            matches!(ray_source.cast_method, RayCastMethod::Screenspace(pos) if pos == CURSOR_POS)
        )
    }

    struct TestOrbitCameraPlugin;

    impl Plugin for TestOrbitCameraPlugin {
        fn build(&self, app: &mut App) {
            app.add_event::<CursorMoved>()
                .add_event::<MouseMotion>()
                .init_resource::<ActionState<ControlAction>>()
                .add_loopless_state(GameState::World)
                .add_plugin(CorePlugin)
                .add_plugin(TimePlugin)
                .add_plugin(OrbitCameraPlugin);
        }
    }
}
