use bevy::prelude::*;

use crate::theme::Theme;

pub(super) struct PopupPlugin;

impl Plugin for PopupPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::init)
            .add_systems(PostUpdate, Self::close);
    }
}

impl PopupPlugin {
    fn init(
        trigger: Trigger<OnAdd, Popup>,
        theme: Res<Theme>,
        window: Single<&Window>,
        mut popups: Query<(&Popup, &mut Node, &mut BackgroundColor), Without<Button>>,
        mut buttons: Query<(&mut GlobalTransform, &mut Node), With<Button>>,
    ) {
        let (popup, mut popup_node, mut background) = popups.get_mut(trigger.entity()).unwrap();
        let (button_transform, button_node) = buttons.get_mut(popup.button_entity).unwrap();

        let (Val::Px(button_width), Val::Px(button_height)) =
            (button_node.width, button_node.height)
        else {
            panic!("button size should be set in pixels");
        };
        let button_pos = button_transform.translation();
        let left = button_pos.x - button_width / 2.0;
        let bottom = window.resolution.height() - button_pos.y + button_height / 2.0;

        popup_node.flex_direction = FlexDirection::Column;
        popup_node.padding = theme.padding.normal;
        popup_node.left = Val::Px(left);
        popup_node.bottom = Val::Px(bottom);
        popup_node.position_type = PositionType::Absolute;
        *background = theme.popup_background;
    }

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

#[derive(Component)]
#[require(Node)]
pub struct Popup {
    pub button_entity: Entity,
}
