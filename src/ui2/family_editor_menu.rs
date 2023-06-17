use bevy::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    theme::Theme,
    widget::{
        button::{ExclusiveButton, Pressed, TextButtonBundle},
        text_edit::TextEditBundle,
        ui_root::UiRoot,
        LabelBundle,
    },
};
use crate::core::{actor::Sex, game_state::GameState};

pub(super) struct FamilyEditorMenuPlugin;

impl Plugin for FamilyEditorMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::setup_system.in_schedule(OnEnter(GameState::FamilyEditor)));
    }
}

impl FamilyEditorMenuPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            size: Size::new(Val::Percent(30.0), Val::Percent(25.0)),
                            flex_direction: FlexDirection::Column,
                            gap: theme.gap.normal,
                            padding: theme.padding.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        // TODO 0.11: Use grid layout
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                const GRID_GAP: Size = Size::all(Val::Px(10.0));
                                parent
                                    .spawn(NodeBundle {
                                        style: Style {
                                            flex_direction: FlexDirection::Column,
                                            gap: GRID_GAP,
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    })
                                    .with_children(|parent| {
                                        parent.spawn(LabelBundle::normal(&theme, "First name"));
                                        parent.spawn(LabelBundle::normal(&theme, "Last name"));
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
                                        parent
                                            .spawn((FirstNameLabel, TextEditBundle::empty(&theme)));
                                        parent
                                            .spawn((LastNameLabel, TextEditBundle::empty(&theme)));
                                    });
                            });

                        parent.spawn(NodeBundle::default()).with_children(|parent| {
                            for (index, sex) in Sex::iter().enumerate() {
                                parent.spawn((
                                    sex,
                                    ExclusiveButton,
                                    Pressed(index == 0),
                                    TextButtonBundle::normal(&theme, sex.to_string()),
                                ));
                            }
                        });
                    });

                parent
                    .spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            position: UiRect::new(
                                Val::Undefined,
                                Val::Px(0.0),
                                Val::Undefined,
                                Val::Px(0.0),
                            ),
                            gap: theme.gap.normal,
                            padding: theme.padding.global,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        for button in FamilyMenuButton::iter() {
                            parent.spawn((
                                button,
                                TextButtonBundle::normal(&theme, button.to_string()),
                            ));
                        }
                    });
            });
    }
}

#[derive(Component)]
struct FirstNameLabel;

#[derive(Component)]
struct LastNameLabel;

#[derive(Component, EnumIter, Clone, Copy, Display)]
enum FamilyMenuButton {
    Confirm,
    Cancel,
}
