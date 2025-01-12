use bevy::prelude::*;
use bevy_simple_text_input::{
    TextInput, TextInputCursorPos, TextInputInactive, TextInputTextColor, TextInputTextFont,
    TextInputValue,
};

use super::theme::Theme;

/// Adds focus functionality to `bevy_simple_text_input`.
pub(super) struct TextEditPlugin;

impl Plugin for TextEditPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::theme)
            .add_systems(PostUpdate, Self::update_border_colors);
    }
}

impl TextEditPlugin {
    fn theme(
        trigger: Trigger<OnAdd, TextEdit>,
        mut commands: Commands,
        theme: Res<Theme>,
        mut text: Query<(
            &mut Node,
            &mut BackgroundColor,
            &mut TextInputTextColor,
            &mut TextInputTextFont,
            &mut TextInputCursorPos,
            &mut TextInputInactive,
            &TextInputValue,
        )>,
        other_edits: Query<(), With<TextEdit>>,
    ) {
        let (
            mut node,
            mut background,
            mut text_color,
            mut text_font,
            mut cursor_pos,
            mut inactive,
            text,
        ) = text.get_mut(trigger.entity()).unwrap();

        node.min_width = theme.text_edit.min_width;
        node.border = theme.text_edit.border;
        node.padding = theme.text_edit.padding;
        *background = theme.text_edit.background_color;
        text_font.0.font = theme.text_edit.font.clone();
        text_font.0.font_size = theme.text_edit.font_size;
        text_color.0 = theme.text_edit.text_color;
        cursor_pos.0 = text.0.len();

        // Activate if the input is single.
        // TODO 0.16: iterate only onver neighbors when hierarchy will be available.
        inactive.0 = other_edits.get_single().is_err();
        commands.entity(trigger.entity()).observe(Self::activate);
    }

    fn update_border_colors(
        theme: Res<Theme>,
        mut text_inputs: Query<(&TextInputInactive, &mut BorderColor), Changed<TextInputInactive>>,
    ) {
        for (inactive, mut border_color) in &mut text_inputs {
            *border_color = if inactive.0 {
                theme.text_edit.inactive_border
            } else {
                theme.text_edit.active_border
            };
        }
    }

    fn activate(trigger: Trigger<Pointer<Click>>, mut text_inputs: Query<&mut TextInputInactive>) {
        // Deactivate others.
        for mut inactive in &mut text_inputs {
            if !inactive.0 {
                inactive.0 = true;
            }
        }

        let mut inactive = text_inputs.get_mut(trigger.entity()).unwrap();
        inactive.0 = false;
    }
}

#[derive(Component, Default)]
#[require(TextInput, TextInputCursorPos)]
pub struct TextEdit;
