use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy_mod_raycast::Ray3d;
use bevy_rapier3d::prelude::*;
use derive_more::From;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{
    asset_metadata,
    control_action::ControlAction,
    errors::log_err_system,
    game_world::{GameEntity, GameWorld},
    preview::PreviewCamera,
};

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ObjectPath>()
            .add_system(
                Self::spawn_scene_system
                    .chain(log_err_system)
                    .run_if_resource_exists::<GameWorld>(),
            )
            .add_system(Self::movement_system.run_if_resource_exists::<GameWorld>())
            .add_system(
                Self::confirm_system
                    .run_if_resource_exists::<GameWorld>()
                    .run_if(is_placement_confirmed),
            )
            .add_system(
                Self::cancel_system
                    .run_if_resource_exists::<GameWorld>()
                    .run_if(is_placement_canceled),
            );
    }
}

impl ObjectPlugin {
    fn spawn_scene_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        spawned_objects: Query<(Entity, &ObjectPath), Added<ObjectPath>>,
    ) -> Result<()> {
        for (entity, object_path) in &spawned_objects {
            let scene_path = asset_metadata::scene_path(&object_path.0)
                .context("Unable to get scene path to spawn object")?;
            let scene_handle: Handle<Scene> = asset_server.load(&scene_path);

            commands
                .entity(entity)
                .insert(scene_handle)
                .insert(GlobalTransform::default())
                .insert_bundle(VisibilityBundle::default());
        }

        Ok(())
    }

    fn movement_system(
        windows: Res<Windows>,
        rapier_ctx: Res<RapierContext>,
        camera: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
        mut moving_objects: Query<&mut Transform, With<MovingObject>>,
    ) {
        if let Ok(mut transform) = moving_objects.get_single_mut() {
            if let Some(cursor_pos) = windows
                .get_primary()
                .and_then(|window| window.cursor_position())
            {
                let (&camera_transform, camera) = camera.single();
                let ray = Ray3d::from_screenspace(cursor_pos, camera, &camera_transform)
                    .expect("Unable to create ray from screenspace");

                let toi = rapier_ctx
                    .cast_ray(
                        ray.origin(),
                        ray.direction(),
                        f32::MAX,
                        false,
                        QueryFilter::new(),
                    )
                    .map(|(_, toi)| toi)
                    .unwrap_or_default();

                transform.translation = ray.origin() + ray.direction() * toi;
            }
        }
    }

    fn cancel_system(mut commands: Commands, moving_objects: Query<Entity, With<MovingObject>>) {
        if let Ok(entity) = moving_objects.get_single() {
            commands.entity(entity).despawn();
        }
    }

    fn confirm_system(mut commands: Commands, moving_objects: Query<Entity, With<MovingObject>>) {
        if let Ok(entity) = moving_objects.get_single() {
            commands.entity(entity).remove::<MovingObject>();
        }
    }
}

fn is_placement_canceled(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.pressed(ControlAction::CancelPlacement)
}

fn is_placement_confirmed(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.pressed(ControlAction::ConfirmPlacement)
}

#[derive(Component)]
pub(crate) struct MovingObject;

#[derive(Bundle, Default)]
pub(crate) struct ObjectBundle {
    pub(crate) path: ObjectPath,
    pub(crate) transform: Transform,
    pub(crate) game_entity: GameEntity,
}

/// Contains path to a an object metadata file.
#[derive(Component, Default, From, Reflect)]
#[reflect(Component)]
pub(crate) struct ObjectPath(String);

#[cfg(test)]
mod tests {
    use bevy::asset::AssetPlugin;

    use super::*;

    #[test]
    fn spawning() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let object = app.world.spawn().insert(ObjectPath(String::default())).id();

        app.update();

        assert!(app.world.entity(object).contains::<Handle<Scene>>());
        assert!(app.world.entity(object).contains::<GlobalTransform>());
        assert!(app.world.entity(object).contains::<Visibility>());
        assert!(app.world.entity(object).contains::<ComputedVisibility>());
    }

    #[test]
    fn confirmation() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let moving_entity = app.world.spawn().insert(MovingObject).id();
        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::ConfirmPlacement);

        app.update();

        assert!(!app.world.entity(moving_entity).contains::<MovingObject>());
    }

    #[test]
    fn cancellation() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let moving_entity = app.world.spawn().insert(MovingObject).id();
        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::CancelPlacement);

        app.update();

        assert!(app.world.get_entity(moving_entity).is_none());
    }

    struct TestMovingObjectPlugin;

    impl Plugin for TestMovingObjectPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<RapierContext>()
                .init_resource::<Windows>()
                .init_resource::<ActionState<ControlAction>>()
                .init_resource::<GameWorld>()
                .add_plugin(AssetPlugin)
                .add_plugin(ObjectPlugin);
        }
    }
}
