use bevy::{prelude::*, ui::FocusPolicy};

use crate::theme::Theme;

pub(super) struct DialogPlugin;

impl Plugin for DialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init);
    }
}

fn init(
    trigger: Trigger<OnAdd, Dialog>,
    theme: Res<Theme>,
    mut dialogs: Query<&mut BackgroundColor>,
) {
    let mut background_color = dialogs.get_mut(trigger.entity()).unwrap();
    *background_color = theme.modal_background;
}

#[derive(Component, Default)]
#[require(
    Node(|| Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..Default::default()
    }),
    FocusPolicy(|| FocusPolicy::Block),
)]
pub struct Dialog;
