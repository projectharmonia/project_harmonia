use bevy::{app::AppExit, prelude::*};
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::core::game_state::GameState;

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, ui_root::UiRoot},
};

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system.in_schedule(OnEnter(GameState::MainMenu)),
            Self::button_system.in_set(OnUpdate(GameState::MainMenu)),
        ));
    }
}

impl MainMenuPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        size: Size::all(Val::Percent(100.0)),
                        align_items: AlignItems::FlexStart,
                        justify_content: JustifyContent::Center,
                        padding: theme.padding.global,
                        gap: theme.gap.large,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                for button in MainMenuButton::iter() {
                    parent.spawn((button, TextButtonBundle::large(&theme, button.to_string())));
                }
            });
    }

    fn button_system(
        mut exit_events: EventWriter<AppExit>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<(&Interaction, &MainMenuButton), Changed<Interaction>>,
    ) {
        for (interaction, button) in &buttons {
            if *interaction == Interaction::Clicked {
                match button {
                    MainMenuButton::Play => game_state.set(GameState::WorldBrowser),
                    MainMenuButton::Settings => game_state.set(GameState::Settings),
                    MainMenuButton::Exit => exit_events.send_default(),
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Display, EnumIter)]
enum MainMenuButton {
    Play,
    Settings,
    Exit,
}
