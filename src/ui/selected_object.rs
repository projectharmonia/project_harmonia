use bevy::{asset::HandleId, prelude::*};
use iyes_loopless::prelude::*;

use crate::core::{
    city::City,
    game_state::GameState,
    object::cursor_object::{self, CursorObject},
};

pub(super) struct SelectedObjectPlugin;

impl Plugin for SelectedObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            // Should run in state before `Self::removal_selection_system` to flush spawn command,
            // otherwise `MovingObject` will be missing and it will be detected as removal.
            CoreStage::PreUpdate,
            Self::cursor_spawning_system
                .run_in_state(GameState::City)
                .run_if_resource_added::<SelectedObject>(),
        )
        .add_system(
            Self::selection_removing_system
                .run_in_state(GameState::City)
                .run_if_resource_exists::<SelectedObject>()
                .run_if_not(cursor_object::is_cursor_object_exists),
        );
    }
}

impl SelectedObjectPlugin {
    fn cursor_spawning_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        selected_object: Res<SelectedObject>,
        visible_cities: Query<Entity, (With<City>, With<Visibility>)>,
    ) {
        let metadata_path = asset_server
            .get_handle_path(selected_object.0)
            .expect("selected object metadata should have a path");

        commands
            .entity(visible_cities.single())
            .with_children(|parent| {
                parent
                    .spawn()
                    .insert(CursorObject::Spawning(metadata_path.path().to_path_buf()));
            });
    }

    fn selection_removing_system(mut commands: Commands) {
        commands.remove_resource::<SelectedObject>();
    }
}

/// Resource that represents object selection in an object placement menu.
#[derive(Clone, Copy)]
pub(super) struct SelectedObject(pub(crate) HandleId);

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy::{asset::AssetPlugin, core::CorePlugin, utils::Uuid};

    use super::*;
    use crate::core::asset_metadata::AssetMetadata;

    #[test]
    fn cursor_spawning() {
        let mut app = App::new();
        app.add_loopless_state(GameState::City)
            .add_plugin(CorePlugin)
            .add_plugin(AssetPlugin)
            .add_plugin(SelectedObjectPlugin);

        app.update();

        let city = app
            .world
            .spawn()
            .insert(City)
            .insert(Visibility::default())
            .id();

        const METADATA_PATH: &str = "dummy.toml";
        let asset_server = app.world.resource::<AssetServer>();
        let dummy_handle: Handle<AssetMetadata> = asset_server.load(METADATA_PATH);
        app.world.insert_resource(SelectedObject(dummy_handle.id));

        app.update();

        let (parent, cursor_object) = app
            .world
            .query::<(&Parent, &CursorObject)>()
            .single(&app.world);

        assert_eq!(parent.get(), city);
        assert!(
            matches!(cursor_object, CursorObject::Spawning(metadata_path) if metadata_path == Path::new(METADATA_PATH))
        );
    }

    #[test]
    fn selection_removing() {
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
