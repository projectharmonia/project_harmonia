use bevy::prelude::*;
use strum::IntoEnumIterator;

use project_harmonia_base::core::GameState;

pub(super) struct UiRootPlugin;

impl Plugin for UiRootPlugin {
    fn build(&self, app: &mut App) {
        for state in GameState::iter() {
            app.add_systems(OnExit(state), Self::despawn);
        }
    }
}

impl UiRootPlugin {
    fn despawn(mut commands: Commands, roots: Query<Entity, With<UiRoot>>) {
        commands.entity(roots.single()).despawn_recursive();
    }
}

#[derive(Component)]
pub(crate) struct UiRoot;
