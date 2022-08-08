use bevy::prelude::*;
use iyes_loopless::prelude::*;
use strum::Display;

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(GameState::Menu);
    }
}

#[derive(Clone, Debug, Display, Eq, Hash, PartialEq)]
pub(crate) enum GameState {
    Menu,
    InGame,
}
