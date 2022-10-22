use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{family::FamilyBundle, game_state::GameState};

pub(super) struct FamilyEditorPlugin;

impl Plugin for FamilyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::FamilyEditor, Self::spawn_system)
            .add_exit_system(GameState::FamilyEditor, Self::cleanup_system);
    }
}

impl FamilyEditorPlugin {
    fn spawn_system(mut commands: Commands) {
        commands
            .spawn_bundle(FamilyEditorBundle::default())
            .with_children(|parent| {
                parent
                    .spawn_bundle(FamilyBundle::default())
                    .insert(EditableFamily);
            });
    }

    fn cleanup_system(mut commands: Commands, family_editors: Query<Entity, With<FamilyEditor>>) {
        commands.entity(family_editors.single()).despawn_recursive();
    }
}

#[derive(Bundle)]
struct FamilyEditorBundle {
    name: Name,
    family_editor: FamilyEditor,

    #[bundle]
    spatial_bundle: SpatialBundle,
}

impl Default for FamilyEditorBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Family editor"),
            family_editor: FamilyEditor,
            spatial_bundle: Default::default(),
        }
    }
}

/// A root family editor component.
#[derive(Component, Default)]
pub(crate) struct FamilyEditor;

/// Currently editing family.
#[derive(Component)]
pub(crate) struct EditableFamily;

/// Currently editing doll.
#[derive(Component)]
pub(crate) struct EditableDoll;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn() {
        let mut app = App::new();
        app.add_loopless_state(GameState::FamilyEditor)
            .add_plugin(FamilyEditorPlugin);

        app.update();

        let family_editor_entity = app
            .world
            .query_filtered::<Entity, With<FamilyEditor>>()
            .single(&app.world);

        app.world.insert_resource(NextState(GameState::MainMenu));

        app.update();

        assert!(app.world.get_entity(family_editor_entity).is_none());
    }
}
