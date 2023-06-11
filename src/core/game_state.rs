use bevy::prelude::*;
use strum::EnumIter;

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>();
    }
}

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Default, EnumIter)]
pub(crate) enum GameState {
    #[default]
    MainMenu,
    Settings,
    WorldBrowser,
    FamilyEditor,
    World,
    City,
    Family,
}
