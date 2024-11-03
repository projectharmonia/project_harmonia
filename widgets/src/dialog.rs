use bevy::{prelude::*, ui::FocusPolicy};

use crate::theme::Theme;

#[derive(Bundle)]
pub struct DialogBundle {
    dialog: Dialog,
    interaction: Interaction,
    node_bundle: NodeBundle,
}

impl DialogBundle {
    pub fn new(theme: &Theme) -> Self {
        Self {
            dialog: Dialog,
            interaction: Default::default(),
            node_bundle: NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                focus_policy: FocusPolicy::Block,
                background_color: theme.modal_color.into(),
                ..Default::default()
            },
        }
    }

    pub fn with_display(mut self, display: Display) -> Self {
        self.node_bundle.style.display = display;
        self
    }
}

#[derive(Component)]
pub struct Dialog;
