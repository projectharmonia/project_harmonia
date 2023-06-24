use bevy::prelude::*;

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, ui_root::UiRoot, DialogBundle, LabelBundle},
};
use crate::core::error::LastError;

pub(super) struct ErrorMessagePlugin;

impl Plugin for ErrorMessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system.run_if(resource_exists_and_changed::<LastError>()),
            Self::button_system,
        ));
    }
}

impl ErrorMessagePlugin {
    // TODO: Use event instead of resource.
    fn setup_system(
        mut commands: Commands,
        theme: Res<Theme>,
        error_message: Res<LastError>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
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
                                format!("Error: {:#}", error_message.0),
                            ));
                            parent.spawn((OkButton, TextButtonBundle::normal(&theme, "Ok")));
                        });
                });
        });
    }

    fn button_system(
        mut commands: Commands,
        buttons: Query<&Interaction, (Changed<Interaction>, With<OkButton>)>,
        dialogs: Query<Entity, With<ErrorDialog>>,
    ) {
        for &interaction in &buttons {
            if interaction == Interaction::Clicked {
                commands.entity(dialogs.single()).despawn_recursive();
            }
        }
    }
}

#[derive(Component)]
struct OkButton;

#[derive(Component)]
struct ErrorDialog;
