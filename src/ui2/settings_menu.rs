use bevy::prelude::*;
use leafwing_input_manager::{
    user_input::{InputKind, UserInput},
    Actionlike,
};
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::core::{action::Action, settings::Settings};

use super::{
    button::{ButtonCommandsExt, ExclusiveButton, Pressed},
    checkbox::CheckboxCommandsExt,
    theme::Theme,
    ui_state::UiState,
};

pub(super) struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system.in_schedule(OnEnter(UiState::Settings)),
            Self::cleanup_system.in_schedule(OnExit(UiState::Settings)),
        ))
        .add_system(Self::display_system.in_set(OnUpdate(UiState::Settings)));
    }
}

impl SettingsMenuPlugin {
    fn setup_system(mut commands: Commands, settings: Res<Settings>, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                SettingsMenu,
            ))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        for (index, tab) in SettingsTab::iter().enumerate() {
                            parent.spawn_button(
                                &theme,
                                tab.to_string(),
                                (tab, ExclusiveButton, Pressed(index == 0)),
                            );
                        }
                    });

                for tab in SettingsTab::iter() {
                    parent
                        .spawn((
                            tab,
                            NodeBundle {
                                style: Style {
                                    margin: theme.tab_content_margin,
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .with_children(|parent| match tab {
                            SettingsTab::Video => setup_video_tab(parent, &theme, &settings),
                            SettingsTab::Controls => setup_controls_tab(parent, &theme, &settings),
                            SettingsTab::Developer => {
                                setup_developer_tab(parent, &theme, &settings)
                            }
                        });
                }
            });
    }

    fn display_system(
        tabs: Query<(&Pressed, &SettingsTab), Changed<Pressed>>,
        mut tab_nodes: Query<(&mut Style, &SettingsTab), Without<Pressed>>,
    ) {
        for (pressed, tab) in &tabs {
            let (mut style, _) = tab_nodes
                .iter_mut()
                .find(|&(_, node_tab)| node_tab == tab)
                .expect("tabs should have associated nodes");
            style.display = if pressed.0 {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    fn cleanup_system(mut commands: Commands, main_menus: Query<Entity, With<SettingsMenu>>) {
        commands.entity(main_menus.single()).despawn_recursive();
    }
}

fn setup_video_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_checkbox(
                theme,
                settings.video.perf_stats,
                "Display performance stats",
                (),
            );
        });
}

fn setup_controls_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    // TODO 0.11: Use grid layout.
    const PADDING: f32 = 7.5;
    parent
        .spawn(NodeBundle {
            style: Style {
                gap: Size::all(Val::Px(PADDING * 2.0)),
                padding: UiRect::all(Val::Px(PADDING)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for action in Action::variants() {
                parent.spawn(TextBundle::from_section(
                    action.to_string(),
                    theme.text.normal.clone(),
                ));
            }
        });

    for index in 0..3 {
        parent
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                for action in Action::variants() {
                    let inputs = settings.controls.mappings.get(action);
                    let button_text = match inputs.get_at(index) {
                        Some(UserInput::Single(InputKind::GamepadButton(gamepad_button))) => {
                            format!("{gamepad_button:?}")
                        }
                        Some(UserInput::Single(InputKind::Keyboard(keycode))) => {
                            format!("{keycode:?}")
                        }
                        Some(UserInput::Single(InputKind::Mouse(mouse_button))) => {
                            format!("{mouse_button:?}")
                        }
                        _ => "Empty".to_string(),
                    };

                    parent.spawn_button(theme, button_text, ());
                }
            });
    }
}

fn setup_developer_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn_checkbox(
                theme,
                settings.developer.game_inspector,
                "Enable game inspector",
                (),
            );
            parent.spawn_checkbox(
                theme,
                settings.developer.debug_collisions,
                "Debug collisions",
                (),
            );
            parent.spawn_checkbox(
                theme,
                settings.developer.debug_paths,
                "Debug navigation paths",
                (),
            );
        });
}

#[derive(Component)]
struct SettingsMenu;

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum SettingsTab {
    Video,
    Controls,
    Developer,
}
