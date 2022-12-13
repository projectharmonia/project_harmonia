use bevy::prelude::*;
use iyes_loopless::prelude::*;
use strum::{Display, EnumIter};

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(GameState::MainMenu)
            .add_loopless_state(CursorMode::Objects);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum GameState {
    MainMenu,
    FamilyEditor,
    World,
    City,
    Family,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter)]
pub(crate) enum CursorMode {
    Objects,
    Lots,
}

impl CursorMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "🌳",
            Self::Lots => "🚧",
        }
    }
}
