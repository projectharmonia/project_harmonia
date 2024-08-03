use bevy::prelude::*;

use crate::theme::Theme;

pub(super) struct PopupPlugin;

impl Plugin for PopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, Self::close);
    }
}

impl PopupPlugin {
    fn close(
        mut commands: Commands,
        popups: Query<(Entity, &Popup)>,
        buttons: Query<&Interaction>,
    ) {
        for (entity, popup) in &popups {
            match buttons.get(popup.button_entity) {
                Ok(Interaction::Hovered) | Ok(Interaction::Pressed) => (),
                _ => commands.entity(entity).despawn_recursive(),
            }
        }
    }
}

#[derive(Bundle)]
pub struct PopupBundle {
    popup: Popup,
    node_bundle: NodeBundle,
}

impl PopupBundle {
    pub fn new(
        theme: &Theme,
        window: &Window,
        button_entity: Entity,
        button_style: &Style,
        button_transform: &GlobalTransform,
    ) -> Self {
        let (Val::Px(button_width), Val::Px(button_height)) =
            (button_style.width, button_style.height)
        else {
            panic!("button size should be set in pixels");
        };
        let button_pos = button_transform.translation();
        let left = button_pos.x - button_width / 2.0;
        let bottom = window.resolution.height() - button_pos.y + button_height / 2.0;

        Self {
            popup: Popup { button_entity },
            node_bundle: NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    padding: theme.padding.normal,
                    left: Val::Px(left),
                    bottom: Val::Px(bottom),
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                background_color: theme.popup_color.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Component)]
struct Popup {
    button_entity: Entity,
}
