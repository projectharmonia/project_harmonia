use bevy::prelude::*;

use super::widget::ui_root::UiRoot;
use crate::core::game_state::GameState;

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system
                .run_if(not(any_with_component::<UiRoot>()))
                .in_schedule(OnEnter(GameState::Family)),
            Self::setup_system
                .run_if(not(any_with_component::<UiRoot>()))
                .in_schedule(OnEnter(GameState::City)),
        ));
    }
}

impl HudPlugin {
    fn setup_system(mut commands: Commands) {
        commands.spawn((
            NodeBundle {
                style: Style {
                    size: Size::all(Val::Percent(100.0)),
                    ..Default::default()
                },
                ..Default::default()
            },
            UiRoot,
        ));
    }
}
