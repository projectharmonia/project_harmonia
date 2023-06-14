use bevy::prelude::*;

use crate::ui2::theme::Theme;

/// A simple stub just to being able to type text.
pub(super) struct TextEditPlugin;

impl Plugin for TextEditPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::input_system);
    }
}

impl TextEditPlugin {
    fn input_system(
        mut char_events: EventReader<ReceivedCharacter>,
        keys: Res<Input<KeyCode>>,
        mut text_edits: Query<&mut Text, With<TextEdit>>,
    ) {
        if let Ok(mut text) = text_edits.get_single_mut() {
            for event in &mut char_events {
                text.sections[0].value.push(event.char);
            }
            if keys.pressed(KeyCode::Back) {
                text.sections[0].value.pop();
            }
        }
    }
}

#[derive(Bundle)]
pub(crate) struct TextEditBundle {
    text_edit: TextEdit,

    #[bundle]
    button_bundle: TextBundle,
}

impl TextEditBundle {
    pub(crate) fn new(theme: &Theme, text: impl Into<String>) -> Self {
        Self {
            text_edit: TextEdit,
            button_bundle: TextBundle {
                style: theme.text_edit.style.clone(),
                text: Text::from_section(text, theme.text_edit.text.clone()),
                background_color: theme.text_edit.background_color.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Component)]
struct TextEdit;
