use bevy::{app::AppExit, prelude::*};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    settings_menu::SettingsMenuOpen,
    theme::Theme,
    widget::{button::TextButtonBundle, click::Click, ui_root::UiRoot},
};
use crate::core::game_state::GameState;

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), Self::setup_system)
            .add_systems(
                Update,
                Self::button_system.run_if(in_state(GameState::MainMenu)),
            );
    }
}

impl MainMenuPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                UiRoot,
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::FlexStart,
                        justify_content: JustifyContent::Center,
                        padding: theme.padding.global,
                        row_gap: theme.gap.large,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                for button in MainMenuButton::iter() {
                    parent.spawn((button, TextButtonBundle::large(&theme, button.to_string())));
                }
            });
    }

    fn button_system(
        mut settings_events: EventWriter<SettingsMenuOpen>,
        mut exit_events: EventWriter<AppExit>,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<&MainMenuButton>,
    ) {
        for event in &mut click_events {
            if let Ok(button) = buttons.get(event.0) {
                match button {
                    MainMenuButton::Play => game_state.set(GameState::WorldBrowser),
                    MainMenuButton::Settings => settings_events.send_default(),
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
