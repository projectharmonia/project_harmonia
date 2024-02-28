use bevy::{prelude::*, ui::UiSystem};
use bevy_simple_text_input::{
    TextInputBundle, TextInputCursorPos, TextInputInactive, TextInputTextStyle, TextInputValue,
};

use crate::ui::theme::Theme;

/// Adds focus functionality to `bevy_simple_text_input`.
pub(super) struct TextEditPlugin;

impl Plugin for TextEditPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, Self::focus_color_system.after(UiSystem::Focus));
    }
}

impl TextEditPlugin {
    fn focus_color_system(
        theme: Res<Theme>,
        interactions: Query<(Entity, &Interaction), Changed<Interaction>>,
        mut text_inputs: Query<(Entity, &mut TextInputInactive, &mut BorderColor)>,
    ) {
        for (interaction_entity, interaction) in &interactions {
            if *interaction == Interaction::Pressed {
                for (input_entity, mut inactive, mut border_color) in &mut text_inputs {
                    if input_entity == interaction_entity {
                        inactive.0 = false;
                        *border_color = theme.text_edit.active_border.into();
                    } else {
                        inactive.0 = true;
                        *border_color = theme.text_edit.inactive_border.into();
                    }
                }
            }
        }
    }
}

#[derive(Bundle)]
pub(crate) struct TextEditBundle {
    node_bundle: NodeBundle,
    text_input_bundle: TextInputBundle,
}

impl TextEditBundle {
    pub(crate) fn new(theme: &Theme, text: impl Into<String>) -> Self {
        let text = text.into();
        Self {
            node_bundle: NodeBundle {
                style: theme.text_edit.style.clone(),
                border_color: theme.text_edit.active_border.into(),
                background_color: theme.text_edit.background_color.into(),
                ..Default::default()
            },
            text_input_bundle: TextInputBundle {
                text_style: TextInputTextStyle(theme.text_edit.text.clone()),
                cursor_pos: TextInputCursorPos(text.len()),
                value: TextInputValue(text),
                ..Default::default()
            },
        }
    }

    pub(crate) fn empty(theme: &Theme) -> Self {
        Self::new(theme, String::new())
    }

    pub(crate) fn inactive(mut self, theme: &Theme) -> Self {
        self.text_input_bundle.inactive.0 = true;
        self.node_bundle.border_color = theme.text_edit.inactive_border.into();
        self
    }
}
