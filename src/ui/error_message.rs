use bevy::prelude::*;

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, ui_root::UiRoot, DialogBundle, LabelBundle},
};
use crate::core::error::ErrorReport;

pub(super) struct ErrorMessagePlugin;

impl Plugin for ErrorMessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((Self::setup_system, Self::button_system));
    }
}

impl ErrorMessagePlugin {
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
                                    size: Size::new(Val::Percent(50.0), Val::Percent(20.0)),
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    gap: theme.gap.normal,
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
        buttons: Query<&Interaction, (Changed<Interaction>, With<OkButton>)>,
        error_dialogs: Query<Entity, With<ErrorDialog>>,
    ) {
        for &interaction in &buttons {
            if interaction == Interaction::Clicked {
                commands.entity(error_dialogs.single()).despawn_recursive();
            }
        }
    }
}

#[derive(Component)]
struct OkButton;

#[derive(Component)]
struct ErrorDialog;
