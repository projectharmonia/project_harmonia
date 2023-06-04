use bevy::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::core::settings::Settings;

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
                            parent.spawn_button(&theme, tab.to_string()).insert((
                                tab,
                                ExclusiveButton,
                                Pressed(index == 0),
                            ));
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
                                parent.spawn_checkbox(
                                    &theme,
                                    settings.video.perf_stats,
                                    "Display performance stats",
                                );
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
