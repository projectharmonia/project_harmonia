use anyhow::{Context, Result};
use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{asset_metadata, errors::log_err_system, game_world::GameWorld};

struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
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
        for (object, object_path) in &spawned_objects {
            let scene_path = asset_metadata::scene_path(&object_path.0)
                .context("Unable to get scene path to spawn object")?;
            let scene: Handle<Scene> = asset_server.load(&scene_path);

            commands
                .entity(object)
                .insert(scene)
                .insert(GlobalTransform::default())
                .insert_bundle(VisibilityBundle::default());
        }

        Ok(())
    }
}

/// Contains path to a an object metadata file.
#[derive(Component)]
struct ObjectPath(String);

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
