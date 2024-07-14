use bevy::prelude::*;

use super::ui_root::UiRoot;
use project_harmonia_base::error_report::ErrorReport;
use project_harmonia_widgets::{
    button::TextButtonBundle, click::Click, dialog::DialogBundle, label::LabelBundle, theme::Theme,
};

pub(super) struct ErrorDialogPlugin;

impl Plugin for ErrorDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Self::show, Self::close));
    }
}

impl ErrorDialogPlugin {
    fn show(
        mut commands: Commands,
        mut error_events: EventReader<ErrorReport>,
        theme: Res<Theme>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for error in error_events.read() {
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

    fn close(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<(), With<OkButton>>,
        error_dialogs: Query<Entity, With<ErrorDialog>>,
    ) {
        for _ in buttons.iter_many(click_events.read().map(|event| event.0)) {
            commands.entity(error_dialogs.single()).despawn_recursive();
        }
    }
}

#[derive(Component)]
struct OkButton;

#[derive(Component)]
struct ErrorDialog;
