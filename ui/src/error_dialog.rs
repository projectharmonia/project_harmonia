use bevy::prelude::*;

use project_harmonia_base::error_message::ErrorMessage;
use project_harmonia_widgets::{
    button::ButtonKind, dialog::Dialog, label::LabelKind, theme::Theme,
};

pub(super) struct ErrorDialogPlugin;

impl Plugin for ErrorDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(show);
    }
}

fn show(
    trigger: Trigger<ErrorMessage>,
    mut commands: Commands,
    theme: Res<Theme>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
) {
    info!("showing error dialog");
    commands.entity(*root_entity).with_children(|parent| {
        parent.spawn(Dialog).with_children(|parent| {
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
                    parent.spawn((LabelKind::Normal, Text::new(&**trigger)));
                    parent.spawn(ButtonKind::Normal).with_child(Text::new("Ok"));
                })
                .observe(close);
        });
    });
}

fn close(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    dialogs: Query<(), With<Dialog>>,
    parents: Query<&Parent>,
) {
    let entity = parents
        .iter_ancestors(trigger.entity())
        .find(|entity| dialogs.get(*entity).is_ok())
        .expect("button should be a part of the error dialog");

    info!("closing error dialog");
    commands.entity(entity).despawn_recursive();
}
