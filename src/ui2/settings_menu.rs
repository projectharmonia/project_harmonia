use bevy::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::core::settings::Settings;

use super::{
    button::{ExclusiveButton, Pressed},
    checkbox::{Checkbox, CheckboxBundle},
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
        .add_system(Self::visibility_system.in_set(OnUpdate(UiState::Settings)));
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
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        for (index, tab) in SettingsTab::iter().enumerate() {
                            parent
                                .spawn((
                                    tab,
                                    ExclusiveButton,
                                    Pressed(index == 0),
                                    ButtonBundle {
                                        style: theme.button.normal.clone(),
                                        background_color: theme.button.normal_color.into(),
                                        ..Default::default()
                                    },
                                ))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle::from_section(
                                        tab.to_string(),
                                        theme.text.normal_button.clone(),
                                    ));
                                });
                        }
                    });

                for tab in SettingsTab::iter() {
                    parent
                        .spawn((
                            tab,
                            NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    margin: UiRect::all(Val::Px(50.0)),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                        ))
                        .with_children(|parent| match tab {
                            SettingsTab::Video => {
                                parent
                                    .spawn(NodeBundle {
                                        style: theme.checkbox.node.clone(),
                                        ..Default::default()
                                    })
                                    .with_children(|parent| {
                                        parent.spawn(CheckboxBundle {
                                            checkbox: Checkbox(settings.video.perf_stats),
                                            button_bundle: ButtonBundle {
                                                style: theme.checkbox.button.clone(),
                                                ..Default::default()
                                            },
                                        });
                                        parent.spawn(TextBundle::from_section(
                                            "Display performance stats",
                                            theme.text.normal.clone(),
                                        ));
                                    });
                            }
                            SettingsTab::Controls => (),
                            SettingsTab::Developer => (),
                        });
                }
            });
    }

    fn visibility_system(
        tabs: Query<(&Pressed, &SettingsTab), Changed<Pressed>>,
        mut tab_nodes: Query<(&mut Visibility, &SettingsTab), Without<Pressed>>,
    ) {
        for (pressed, tab) in &tabs {
            let (mut visibility, _) = tab_nodes
                .iter_mut()
                .find(|&(_, node_tab)| node_tab == tab)
                .expect("tabs should have associated nodes");
            *visibility = if pressed.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }

    fn cleanup_system(mut commands: Commands, main_menus: Query<Entity, With<SettingsMenu>>) {
        commands.entity(main_menus.single()).despawn_recursive();
    }
}

#[derive(Component)]
struct SettingsMenu;

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum SettingsTab {
    Video,
    Controls,
    Developer,
}
