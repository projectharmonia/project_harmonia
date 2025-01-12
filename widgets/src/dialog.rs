use bevy::{prelude::*, ui::FocusPolicy};

use crate::theme::Theme;

pub(super) struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::init);
    }
}

impl DialogPlugin {
    fn init(
        trigger: Trigger<OnAdd, Dialog>,
        theme: Res<Theme>,
        mut dialogs: Query<(&mut Node, &mut FocusPolicy, &mut BackgroundColor)>,
    ) {
        let (mut node, mut focus_policy, mut background_color) =
            dialogs.get_mut(trigger.entity()).unwrap();

        node.position_type = PositionType::Absolute;
        node.width = Val::Percent(100.0);
        node.height = Val::Percent(100.0);
        node.align_items = AlignItems::Center;
        node.justify_content = JustifyContent::Center;

        *focus_policy = FocusPolicy::Block;
        *background_color = theme.modal_background;
    }
}

#[derive(Component, Default)]
#[require(Node)]
pub struct Dialog;
