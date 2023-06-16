use bevy::prelude::*;

use crate::ui2::theme::Theme;

/// A simple stub just to being able to type text.
pub(super) struct TextEditPlugin;

impl Plugin for TextEditPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::input_system,
            Self::interaction_system,
            Self::activation_system,
            Self::exclusive_system,
        ));
    }
}

impl TextEditPlugin {
    fn input_system(
        mut char_events: EventReader<ReceivedCharacter>,
        keys: Res<Input<KeyCode>>,
        mut text_edits: Query<(&mut Text, &TextEdit)>,
    ) {
        if let Some(mut text) = text_edits
            .iter_mut()
            .find_map(|(text, text_edit)| text_edit.active.then_some(text))
        {
            for event in &mut char_events {
                text.sections[0].value.push(event.char);
            }
            if keys.pressed(KeyCode::Back) {
                text.sections[0].value.pop();
            }
        }
    }

    fn interaction_system(
        theme: Res<Theme>,
        mut text_edits: Query<
            (&Interaction, &mut BackgroundColor, &TextEdit),
            Or<(Changed<Interaction>, Changed<TextEdit>)>,
        >,
    ) {
        for (&interaction, mut background, text_edit) in &mut text_edits {
            *background = match (interaction, text_edit.active) {
                (Interaction::Clicked, _) | (Interaction::None, true) => {
                    theme.text_edit.active_color.into()
                }
                (Interaction::Hovered, true) => theme.text_edit.hovered_active_color.into(),
                (Interaction::Hovered, false) => theme.text_edit.hovered_color.into(),
                (Interaction::None, false) => theme.text_edit.normal_color.into(),
            };
        }
    }

    fn activation_system(
        mut text_edits: Query<(&Interaction, &mut TextEdit), Changed<Interaction>>,
    ) {
        for (&interaction, mut text_edit) in &mut text_edits {
            if interaction == Interaction::Clicked {
                text_edit.active = true;
            }
        }
    }

    fn exclusive_system(mut text_edits: Query<(Entity, &mut TextEdit)>) {
        if let Some((active_entity, _)) = text_edits
            .iter_mut()
            .find(|(_, text_edit)| text_edit.is_changed() && text_edit.active)
        {
            for (text_entity, mut text_edit) in &mut text_edits {
                if text_edit.active && text_entity != active_entity {
                    text_edit.active = false;
                }
            }
        }
    }
}

#[derive(Bundle)]
pub(crate) struct TextEditBundle {
    text_edit: TextEdit,
    interaction: Interaction,

    #[bundle]
    button_bundle: TextBundle,
}

impl TextEditBundle {
    pub(crate) fn new(theme: &Theme, text: impl Into<String>) -> Self {
        Self {
            text_edit: Default::default(),
            interaction: Default::default(),
            button_bundle: TextBundle {
                style: theme.text_edit.style.clone(),
                text: Text::from_section(text, theme.text_edit.text.clone()),
                ..Default::default()
            },
        }
    }

    pub(crate) fn active(mut self) -> Self {
        self.text_edit.active = true;
        self
    }
}

#[derive(Component, Default)]
struct TextEdit {
    active: bool,
}
