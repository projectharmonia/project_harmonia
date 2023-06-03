use bevy::{app::AppExit, prelude::*};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{theme::Theme, ui_state::UiState};

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system.in_schedule(OnEnter(UiState::MainMenu)),
            Self::button_system.in_set(OnUpdate(UiState::MainMenu)),
            Self::cleanup_system.in_schedule(OnExit(UiState::MainMenu)),
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
                        ..Default::default()
                    },
                    ..Default::default()
                },
                MainMenu,
            ))
            .with_children(|parent| {
                for button in MainMenuButton::iter() {
                    parent
                        .spawn((
                            ButtonBundle {
                                style: theme.button.large.clone(),
                                background_color: theme.button.normal_color.into(),
                                ..Default::default()
                            },
                            button,
                        ))
                        .with_children(|parent| {
                            parent.spawn(TextBundle::from_section(
                                button.to_string(),
                                theme.text.large_button.clone(),
                            ));
                        });
                }
            });
    }

    fn button_system(
        mut exit_events: EventWriter<AppExit>,
        mut ui_state: ResMut<NextState<UiState>>,
        buttons: Query<(&Interaction, &MainMenuButton), Changed<Interaction>>,
    ) {
        for (interaction, button) in &buttons {
            if *interaction == Interaction::Clicked {
                match button {
                    MainMenuButton::Play => ui_state.set(UiState::WorldBrowser),
                    MainMenuButton::Settings => ui_state.set(UiState::Settings),
                    MainMenuButton::Exit => exit_events.send_default(),
                }
            }
        }
    }

    fn cleanup_system(mut commands: Commands, main_menus: Query<Entity, With<MainMenu>>) {
        commands.entity(main_menus.single()).despawn_recursive();
    }
}

#[derive(Component)]
struct MainMenu;

#[derive(Clone, Component, Copy, Display, EnumIter)]
enum MainMenuButton {
    Play,
    Settings,
    Exit,
}
