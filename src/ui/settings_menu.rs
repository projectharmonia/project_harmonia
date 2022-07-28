mod developer_tab;
mod video_tab;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Area, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{back_button::BackButton, ui_action::UiAction, ui_state::UiState, UI_MARGIN};
use crate::core::{
    game_state::GameState,
    settings::{Settings, SettingsApplied},
};
use developer_tab::DeveloperTab;
use video_tab::VideoTab;

pub(super) struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::settings_menu_system.run_in_state(UiState::SettingsMenu))
            .add_system(Self::buttons_system.run_in_state(UiState::SettingsMenu))
            .add_system(Self::back_system.run_in_state(UiState::SettingsMenu));
    }
}

impl SettingsMenuPlugin {
    fn settings_menu_system(
        mut current_tab: Local<SettingsTab>,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
    ) {
        let window_width_margin = egui.ctx_mut().style().spacing.window_margin.left * 2.0;
        let screen_rect = egui.ctx_mut().input().screen_rect();

        Window::new("Settings")
            .anchor(Align2::CENTER_CENTER, (0.0, 0.0))
            .collapsible(false)
            .resizable(false)
            .default_width(screen_rect.width() - UI_MARGIN * 2.0 - window_width_margin)
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    for tab in SettingsTab::iter() {
                        ui.selectable_value(&mut *current_tab, tab, tab.to_string());
                    }
                });
                match *current_tab {
                    SettingsTab::Video => VideoTab::new(&mut settings.video).show(ui),
                    SettingsTab::Developer => DeveloperTab::new(&mut settings.developer).show(ui),
                };
                ui.expand_to_include_rect(ui.available_rect_before_wrap());
            });
    }

    fn buttons_system(
        mut apply_events: EventWriter<SettingsApplied>,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
        mut action_state: ResMut<ActionState<UiAction>>,
    ) {
        Area::new("Settings buttons area")
            .anchor(Align2::RIGHT_BOTTOM, (-UI_MARGIN, -UI_MARGIN))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Restore defaults").clicked() {
                        *settings = Settings::default();
                        apply_events.send(SettingsApplied);
                    }
                    if ui.button("Apply").clicked() {
                        apply_events.send(SettingsApplied);
                    }
                    if ui.button("Ok").clicked() {
                        apply_events.send(SettingsApplied);
                        action_state.press(UiAction::Back);
                    }
                })
            });
    }

    fn back_system(
        mut commands: Commands,
        game_state: Res<CurrentState<GameState>>,
        mut egui: ResMut<EguiContext>,
        mut action_state: ResMut<ActionState<UiAction>>,
    ) {
        if BackButton::new(&mut action_state)
            .show(egui.ctx_mut())
            .clicked()
        {
            let state = match game_state.0 {
                GameState::Menu => UiState::MainMenu,
                GameState::InGame => UiState::InGameMenu,
            };
            commands.insert_resource(NextState(state));
        }
    }
}

#[derive(Default, Display, Clone, Copy, EnumIter, PartialEq)]
enum SettingsTab {
    #[default]
    Video,
    Developer,
}
