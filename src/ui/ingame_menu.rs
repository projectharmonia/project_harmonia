use bevy::{app::AppExit, prelude::*};
use bevy_egui::EguiContext;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use crate::core::game_state::GameState;

use super::{
    modal_window::ModalWindow, settings_menu::SettingsMenu, ui_action::UiAction,
    world_menu::WorldMenu,
};

pub(super) struct InGameMenuPlugin;

impl Plugin for InGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::open_ingame_menu_system
                .run_in_state(GameState::InGame)
                .run_unless_resource_exists::<InGameMenu>(),
        )
        .add_system(Self::ingame_menu_system.run_if_resource_exists::<InGameMenu>());
    }
}

impl InGameMenuPlugin {
    fn ingame_menu_system(
        mut commands: Commands,
        mut exit_events: EventWriter<AppExit>,
        mut egui: ResMut<EguiContext>,
        action_state: Res<ActionState<UiAction>>,
    ) {
        let mut open = true;
        ModalWindow::new(&mut open, &action_state, "Menu").show(egui.ctx_mut(), |ui| {
            ui.vertical_centered(|ui| {
                if ui.button("Save").clicked() {}
                if ui.button("Save as ...").clicked() {}
                if ui.button("Settings").clicked() {
                    commands.init_resource::<SettingsMenu>();
                }
                if ui.button("Exit to main menu").clicked() {
                    commands.remove_resource::<InGameMenu>();
                    commands.remove_resource::<WorldMenu>();
                    commands.insert_resource(NextState(GameState::Menu));
                }
                if ui.button("Exit game").clicked() {
                    exit_events.send(AppExit);
                }
            });
        });

        if !open {
            commands.remove_resource::<InGameMenu>();
        }
    }

    fn open_ingame_menu_system(mut commands: Commands, action_state: Res<ActionState<UiAction>>) {
        if action_state.just_pressed(UiAction::Back) {
            commands.init_resource::<InGameMenu>();
        }
    }
}

#[derive(Default)]
struct InGameMenu;
