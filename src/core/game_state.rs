use bevy::prelude::*;

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<GameState>();
    }
}

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Default)]
pub(crate) enum GameState {
    #[default]
    MainMenu,
    FamilyEditor,
    World,
    City,
    Family,
}
