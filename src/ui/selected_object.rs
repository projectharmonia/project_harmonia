use bevy::{asset::HandleId, prelude::*};
use iyes_loopless::prelude::*;

use crate::core::{
    asset_metadata,
    game_state::GameState,
    object::{MovingObject, ObjectBundle},
};

pub(super) struct SelectedObjectPlugin;

impl Plugin for SelectedObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            // Should run in state before `Self::removal_selection_system` to flush spawn command,
            // otherwise `MovingObject` will be missing and it will be detected as removal.
            CoreStage::PreUpdate,
            Self::spawn_selection_system
                .run_in_state(GameState::City)
                .run_if_resource_added::<SelectedObject>(),
        )
        .add_system(
            Self::remove_selection_system
                .run_in_state(GameState::City)
                .run_if_resource_exists::<SelectedObject>(),
        );
    }
}

impl SelectedObjectPlugin {
    fn spawn_selection_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        selected_object: Res<SelectedObject>,
    ) {
        let metadata_path = asset_server
            .get_handle_path(selected_object.0)
            .expect("selected object metadata should have a path");

        let scene_path = asset_metadata::scene_path(metadata_path.path());

        // TODO: Spawn it as a child of the selected city
        // once https://github.com/IyesGames/iyes_scene_tools/issues/5 will be fixed.
        commands
            .spawn_bundle(ObjectBundle {
                path: scene_path.into(),
                ..Default::default()
            })
            .insert(MovingObject);
    }

    fn remove_selection_system(
        mut commands: Commands,
        moving_objects: Query<(), With<MovingObject>>,
    ) {
        if moving_objects.is_empty() {
            commands.remove_resource::<SelectedObject>();
        }
    }
}

/// Resource that represents object selection in an object placement menu.
#[derive(Clone, Copy)]
pub(super) struct SelectedObject(pub(crate) HandleId);

#[cfg(test)]
mod tests {
    use bevy::{asset::AssetPlugin, core::CorePlugin, utils::Uuid};

    use crate::core::{asset_metadata::AssetMetadata, object::ObjectPath};

    use super::*;

    #[test]
    fn spawning_selection() {
        let mut app = App::new();
        app.add_loopless_state(GameState::City)
            .add_plugin(CorePlugin)
            .add_plugin(AssetPlugin)
            .add_plugin(SelectedObjectPlugin);

        app.update();

        let asset_server = app.world.resource::<AssetServer>();
        let dummy_handle: Handle<AssetMetadata> = asset_server.load("dummy.toml");
        app.world.insert_resource(SelectedObject(dummy_handle.id));

        app.update();

        app.world
            .query_filtered::<(), (With<MovingObject>, With<ObjectPath>)>()
            .single(&app.world);
    }

    #[test]
    fn removing_selection() {
        let mut app = App::new();
        app.add_loopless_state(GameState::City)
            .insert_resource(SelectedObject(HandleId::Id(Uuid::nil(), 0)))
            .add_plugin(SelectedObjectPlugin);

        app.update();

        assert!(
            app.world.get_resource::<SelectedObject>().is_none(),
            "selection should be removed when there is no moving object"
        );
    }
}
