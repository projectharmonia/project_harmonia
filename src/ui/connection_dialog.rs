use bevy::prelude::*;
use bevy_replicon::{client_just_disconnected, prelude::*};

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, click::Click, ui_root::UiRoot, DialogBundle, LabelBundle},
};

pub(super) struct ConnectionDialogPlugin;

impl Plugin for ConnectionDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::read_clicks).add_systems(
            PostUpdate,
            (
                Self::show.run_if(resource_added::<RenetClient>),
                Self::close.run_if(client_just_disconnected),
            ),
        );
    }
}

impl ConnectionDialogPlugin {
    fn show(mut commands: Commands, theme: Res<Theme>, roots: Query<Entity, With<UiRoot>>) {
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((ConnectionDialog, DialogBundle::new(&theme)))
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
                            parent.spawn(LabelBundle::normal(&theme, "Connecting to server"));
                            parent
                                .spawn((CancelButton, TextButtonBundle::normal(&theme, "Cancel")));
                        });
                });
        });
    }

    fn read_clicks(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<(), With<CancelButton>>,
        dialogs: Query<Entity, With<ConnectionDialog>>,
    ) {
        for _ in buttons.iter_many(click_events.read().map(|event| event.0)) {
            commands.remove_resource::<RenetClient>();
            commands.entity(dialogs.single()).despawn_recursive();
        }
    }

    fn close(mut commands: Commands, dialogs: Query<Entity, With<ConnectionDialog>>) {
        commands.entity(dialogs.single()).despawn_recursive();
    }
}

#[derive(Component)]
struct CancelButton;

#[derive(Component)]
struct ConnectionDialog;
