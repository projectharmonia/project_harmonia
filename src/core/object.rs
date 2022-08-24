use anyhow::{Context, Result};
use bevy::prelude::*;
use derive_more::From;
use iyes_loopless::prelude::*;

use super::{
    asset_metadata,
    errors::log_err_system,
    game_world::{GameEntity, GameWorld},
};

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<ObjectPath>().add_system(
            Self::spawn_scene_system
                .chain(log_err_system)
                .run_if_resource_exists::<GameWorld>(),
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
}

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
        app.init_resource::<GameWorld>()
            .add_plugin(AssetPlugin)
            .add_plugin(ObjectPlugin);

        let object = app.world.spawn().insert(ObjectPath(String::default())).id();

        app.update();

        assert!(app.world.entity(object).contains::<Handle<Scene>>());
        assert!(app.world.entity(object).contains::<GlobalTransform>());
        assert!(app.world.entity(object).contains::<Visibility>());
        assert!(app.world.entity(object).contains::<ComputedVisibility>());
    }
}
