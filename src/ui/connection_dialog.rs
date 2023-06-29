use bevy::prelude::*;
use bevy_replicon::prelude::*;

use super::{
    theme::Theme,
    widget::{button::TextButtonBundle, click::Click, ui_root::UiRoot, DialogBundle, LabelBundle},
};

pub(super) struct ConnectionDialogPlugin;

impl Plugin for ConnectionDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system.in_schedule(OnEnter(ClientState::Connecting)),
            Self::button_system,
            Self::cleanup_system.in_schedule(OnExit(ClientState::Connecting)),
        ));
    }
}

impl ConnectionDialogPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>, roots: Query<Entity, With<UiRoot>>) {
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((ConnectionDialog, DialogBundle::new(&theme)))
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
                            parent.spawn(LabelBundle::normal(&theme, "Connecting to server"));
                            parent
                                .spawn((CancelButton, TextButtonBundle::normal(&theme, "Cancel")));
                        });
                });
        });
    }

    fn button_system(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        buttons: Query<(), With<CancelButton>>,
        dialogs: Query<Entity, With<ConnectionDialog>>,
    ) {
        for event in &mut click_events {
            if buttons.get(event.0).is_ok() {
                commands.remove_resource::<RenetClient>();
                commands.entity(dialogs.single()).despawn_recursive();
            }
        }
    }

    fn cleanup_system(mut commands: Commands, dialogs: Query<Entity, With<ConnectionDialog>>) {
        commands.entity(dialogs.single()).despawn_recursive();
    }
}

#[derive(Component)]
struct CancelButton;

#[derive(Component)]
struct ConnectionDialog;
