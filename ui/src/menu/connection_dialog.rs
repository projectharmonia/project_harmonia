use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::renet::RenetClient;

use project_harmonia_widgets::{
    button::ButtonKind, dialog::Dialog, label::LabelKind, theme::Theme,
};

pub(super) struct ConnectionDialogPlugin;

impl Plugin for ConnectionDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::show.run_if(client_started_connecting),
                Self::close
                    // Dialog may not be created if the connection happens instantly.
                    .never_param_warn()
                    .run_if(client_just_disconnected.or(client_just_connected)),
            ),
        );
    }
}

impl ConnectionDialogPlugin {
    fn show(
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("showing connection dialog");
        commands.entity(*root_entity).with_children(|parent| {
            parent.spawn(ConnectionDialog).with_children(|parent| {
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        },
                        theme.panel_background,
                    ))
                    .with_children(|parent| {
                        parent.spawn((LabelKind::Normal, Text::new("Connecting to server")));
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Cancel"))
                            .observe(Self::cancel);
                    });
            });
        });
    }

    fn cancel(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
        info!("cancelling connection");
        commands.remove_resource::<RenetClient>();
    }

    fn close(mut commands: Commands, dialog_entity: Single<Entity, With<ConnectionDialog>>) {
        info!("closing connection dialog");
        commands.entity(*dialog_entity).despawn_recursive();
    }
}

#[derive(Component)]
#[require(Dialog)]
struct ConnectionDialog;
