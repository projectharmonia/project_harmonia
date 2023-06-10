use bevy::prelude::*;
use derive_more::Display;
use strum::{EnumIter, IntoEnumIterator};

use crate::core::game_paths::GamePaths;

use super::{
    theme::Theme,
    ui_state::UiState,
    widget::{button::TextButtonBundle, ui_root::UiRoot, LabelBundle},
};

pub(super) struct WorldBrowserPlugin;

impl Plugin for WorldBrowserPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::setup_system.in_schedule(OnEnter(UiState::WorldBrowser)));
    }
}

impl WorldBrowserPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>, game_paths: Res<GamePaths>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        size: Size::all(Val::Percent(100.0)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::FlexStart,
                        padding: theme.padding.global,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                parent.spawn(LabelBundle::large(&theme, "World browser"));

                let world_names = game_paths
                    .get_world_names()
                    .map_err(|e| error!("unable to get world names: {e}"))
                    .unwrap_or_default();
                for world_name in world_names {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                size: Size::new(Val::Percent(50.0), Val::Percent(25.0)),
                                padding: theme.padding.normal,
                                ..Default::default()
                            },
                            background_color: theme.panel_color.into(),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        size: Size::all(Val::Percent(100.0)),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with_children(|parent| {
                                    parent.spawn(LabelBundle::large(&theme, world_name));
                                });
                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Column,
                                        gap: theme.gap.normal,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                })
                                .with_children(|parent| {
                                    for button in WorldButton::iter() {
                                        parent.spawn(TextButtonBundle::normal(
                                            &theme,
                                            button.to_string(),
                                        ));
                                    }
                                });
                        });
                }
            });
    }
}

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum WorldButton {
    Play,
    Host,
    Delete,
}
