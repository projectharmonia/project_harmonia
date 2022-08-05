use bevy::prelude::*;
use iyes_loopless::prelude::*;

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(GameState::Menu);
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum GameState {
    Menu,
    InGame,
}
