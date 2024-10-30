use bevy::{app::AppExit, prelude::*};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{settings_menu::SettingsMenuOpen, MenuState};
use project_harmonia_widgets::{button::TextButtonBundle, click::Click, theme::Theme};

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuState::MainMenu), Self::setup)
            .add_systems(
                Update,
                Self::handle_clicks.run_if(in_state(MenuState::MainMenu)),
            );
    }
}

impl MainMenuPlugin {
    fn setup(
        mut commands: Commands,
        theme: Res<Theme>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("entering main menu");
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(MenuState::MainMenu),
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
        });
    }

    fn handle_clicks(
        mut settings_events: EventWriter<SettingsMenuOpen>,
        mut exit_events: EventWriter<AppExit>,
        mut click_events: EventReader<Click>,
        mut menu_state: ResMut<NextState<MenuState>>,
        buttons: Query<&MainMenuButton>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                MainMenuButton::Play => menu_state.set(MenuState::WorldBrowser),
                MainMenuButton::Settings => {
                    settings_events.send_default();
                }
                MainMenuButton::Exit => {
                    info!("exiting game");
                    exit_events.send_default();
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
