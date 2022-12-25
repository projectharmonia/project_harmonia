use bevy::prelude::*;
use bevy_egui::EguiContext;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;

use super::modal_window::{ModalUiExt, ModalWindow};
use crate::core::{action::Action, error_message::ErrorMessage};

pub(super) struct ErrorMessagePlugin;

impl Plugin for ErrorMessagePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::error_message_system.run_if_resource_exists::<ErrorMessage>());
    }
}

impl ErrorMessagePlugin {
    fn error_message_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<Action>>,
        error_message: Res<ErrorMessage>,
    ) {
        let mut open = true;
        ModalWindow::new("Error")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label(format!("Error: {:#}", error_message.0));
                if ui.button("Ok").clicked() {
                    ui.close_modal();
                }
            });

        if !open {
            commands.remove_resource::<ErrorMessage>();
        }
    }
}
