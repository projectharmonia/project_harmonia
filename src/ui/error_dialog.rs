use bevy::prelude::*;

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, click::Click, ui_root::UiRoot, DialogBundle, LabelBundle},
};
use crate::core::error_report::ErrorReport;

pub(super) struct ErrorDialogPlugin;

impl Plugin for ErrorDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Self::setup_system, Self::button_system));
    }
}

impl ErrorDialogPlugin {
    fn setup_system(
        mut commands: Commands,
        mut error_events: EventReader<ErrorReport>,
        theme: Res<Theme>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for error in &mut error_events {
            commands.entity(roots.single()).with_children(|parent| {
                parent
                    .spawn((ErrorDialog, DialogBundle::new(&theme)))
                    .with_children(|parent| {
                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    padding: theme.padding.normal,
                                    row_gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                background_color: theme.panel_color.into(),
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                parent.spawn(LabelBundle::normal(
                                    &theme,
                                    format!("Error: {:#}", error.0),
                                ));
                                parent.spawn((OkButton, TextButtonBundle::normal(&theme, "Ok")));
                            });
                    });
            });
        }
    }

    fn button_system(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<(), With<OkButton>>,
        error_dialogs: Query<Entity, With<ErrorDialog>>,
    ) {
        for event in &mut click_events {
            if buttons.get(event.0).is_ok() {
                commands.entity(error_dialogs.single()).despawn_recursive();
            }
        }
    }
}

#[derive(Component)]
struct OkButton;

#[derive(Component)]
struct ErrorDialog;
