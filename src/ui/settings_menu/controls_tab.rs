use bevy::prelude::Commands;
use bevy_egui::egui::{Grid, Ui};
use leafwing_input_manager::{prelude::*, user_input::InputKind};

use crate::{
    core::{action::Action, settings::ControlsSettings},
    ui::settings_menu::ActiveBinding,
};

pub(super) struct ControlsTab<'a> {
    controls_settings: &'a mut ControlsSettings,
}

impl<'a> ControlsTab<'a> {
    #[must_use]
    pub(super) fn new(controls_settings: &'a mut ControlsSettings) -> Self {
        Self { controls_settings }
    }
}

impl ControlsTab<'_> {
    pub(super) fn show(self, ui: &mut Ui, commands: &mut Commands) {
        const INPUT_VARIANTS: usize = 3;
        const COLUMNS_COUNT: usize = INPUT_VARIANTS + 1;
        let window_width_margin = ui.style().spacing.window_margin.left * 2.0;

        Grid::new("Controls grid")
            .num_columns(COLUMNS_COUNT)
            .striped(true)
            .min_col_width(ui.available_width() / COLUMNS_COUNT as f32 - window_width_margin)
            .show(ui, |ui| {
                for action in Action::variants() {
                    ui.label(action.to_string());
                    let inputs = self.controls_settings.mappings.get(action);
                    for index in 0..INPUT_VARIANTS {
                        let button_text = match inputs.get_at(index) {
                            Some(UserInput::Single(InputKind::GamepadButton(gamepad_button))) => {
                                format!("🎮 {:?}", gamepad_button)
                            }
                            Some(UserInput::Single(InputKind::Keyboard(keycode))) => {
                                format!("🖮 {:?}", keycode)
                            }
                            Some(UserInput::Single(InputKind::Mouse(mouse_button))) => {
                                format!("🖱 {:?}", mouse_button)
                            }
                            _ => "Empty".to_string(),
                        };
                        if ui.button(button_text).clicked() {
                            commands.insert_resource(ActiveBinding::new(action, index));
                        }
                    }
                    ui.end_row();
                }
            });
    }
}
