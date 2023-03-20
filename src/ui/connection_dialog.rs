use bevy::prelude::*;
use bevy_egui::EguiContexts;
use bevy_renet::renet::RenetClient;
use leafwing_input_manager::prelude::ActionState;

use super::modal_window::ModalWindow;
use crate::core::{
    action::Action,
    network::{client::ConnectionSettings, sets::NetworkSet},
};

pub(super) struct ConnectionDialogPlugin;

impl Plugin for ConnectionDialogPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::connection_system.in_set(NetworkSet::ClientConnecting));
    }
}

impl ConnectionDialogPlugin {
    fn connection_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        mut action_state: ResMut<ActionState<Action>>,
        connection_setting: Res<ConnectionSettings>,
    ) {
        let mut open = true;
        ModalWindow::new("Connection")
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label(format!(
                    "Connecting to {}:{}...",
                    connection_setting.ip, connection_setting.port
                ));
                if ui.button("Cancel").clicked() {
                    commands.remove_resource::<RenetClient>();
                }
            });

        if !open {
            commands.remove_resource::<RenetClient>();
        }
    }
}
