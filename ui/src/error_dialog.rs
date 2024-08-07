use bevy::prelude::*;

use project_harmonia_base::message::Message;
use project_harmonia_widgets::{
    button::TextButtonBundle, click::Click, dialog::DialogBundle, label::LabelBundle, theme::Theme,
};

pub(super) struct MessageBoxPlugin;

impl Plugin for MessageBoxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Self::show, Self::close));
    }
}

impl MessageBoxPlugin {
    fn show(
        mut commands: Commands,
        mut messages: EventReader<Message>,
        theme: Res<Theme>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        for message in messages.read() {
            info!("showing dialog");
            commands.entity(roots.single()).with_children(|parent| {
                parent
                    .spawn((MessageBox, DialogBundle::new(&theme)))
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
                                parent.spawn(LabelBundle::normal(&theme, message.0.clone()));
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
        message_boxes: Query<Entity, With<MessageBox>>,
    ) {
        for _ in buttons.iter_many(click_events.read().map(|event| event.0)) {
            info!("closing dialog");
            commands.entity(message_boxes.single()).despawn_recursive();
        }
    }
}

#[derive(Component)]
struct OkButton;

#[derive(Component)]
struct MessageBox;
