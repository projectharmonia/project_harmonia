use bevy::prelude::*;
use strum::IntoEnumIterator;

use crate::core::game_state::GameState;

pub(super) struct UiRootPlugin;

impl Plugin for UiRootPlugin {
    fn build(&self, app: &mut App) {
        for state in GameState::iter() {
            app.add_systems(OnExit(state), Self::cleanup_system);
        }
    }
}

impl UiRootPlugin {
    fn cleanup_system(mut commands: Commands, roots: Query<Entity, With<UiRoot>>) {
        commands.entity(roots.single()).despawn_recursive();
    }
}

#[derive(Component)]
pub(crate) struct UiRoot;
