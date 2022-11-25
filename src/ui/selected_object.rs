use bevy::{asset::HandleId, prelude::*};
use iyes_loopless::prelude::*;

use crate::core::{
    city::ActiveCity,
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
                .run_if_resource_added::<SelectedObject>()
                .run_in_state(GameState::City),
        )
        .add_system(
            Self::selection_removing_system
                .run_if_not(cursor_object::cursor_object_exists)
                .run_if_resource_exists::<SelectedObject>()
                .run_in_state(GameState::City),
        );
    }
}

impl SelectedObjectPlugin {
    fn cursor_spawning_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        selected_object: Res<SelectedObject>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        let metadata_path = asset_server
            .get_handle_path(selected_object.0)
            .expect("selected object metadata should have a path");

        commands
            .entity(active_cities.single())
            .with_children(|parent| {
                parent.spawn(CursorObject::Spawning(metadata_path.path().to_path_buf()));
            });
    }

    fn selection_removing_system(mut commands: Commands) {
        commands.remove_resource::<SelectedObject>();
    }
}

/// Resource that represents object selection in an object placement menu.
#[derive(Clone, Copy, Resource)]
pub(super) struct SelectedObject(pub(crate) HandleId);
