use bevy::prelude::*;
use strum::EnumIter;

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>();
    }
}

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Default, EnumIter)]
pub enum GameState {
    #[default]
    MainMenu,
    WorldBrowser,
    FamilyEditor,
    World,
    City,
    Family,
}
