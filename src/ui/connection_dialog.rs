use bevy::prelude::*;
use bevy_egui::EguiContext;
use bevy_renet::renet::RenetClient;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{modal_window::ModalWindow, ui_action::UiAction};
use crate::core::network::client::{self, ConnectionSettings};

pub(super) struct ConnectionDialogPlugin;

impl Plugin for ConnectionDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::connection_dialog_system.run_if(client::is_connecting));
    }
}

impl ConnectionDialogPlugin {
    fn connection_dialog_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
        connection_setting: Res<ConnectionSettings>,
    ) {
        let mut is_open = true;
        ModalWindow::new("Connection")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label(format!(
                    "Connecting to {}:{}...",
                    connection_setting.ip, connection_setting.port
                ));
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<RenetClient>();
                }
            });

        if !is_open {
            commands.remove_resource::<RenetClient>();
        }
    }
}
