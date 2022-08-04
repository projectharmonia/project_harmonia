mod developer_tab;
mod video_tab;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align, Layout},
    EguiContext,
};
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{modal_window::ModalWindow, ui_action::UiAction};
use crate::core::settings::{Settings, SettingsApplied};
use developer_tab::DeveloperTab;
use video_tab::VideoTab;

pub(super) struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::settings_menu_system.run_if_resource_exists::<SettingsMenu>());
    }
}

impl SettingsMenuPlugin {
    fn settings_menu_system(
        mut commands: Commands,
        mut apply_events: EventWriter<SettingsApplied>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut settings_menu: ResMut<SettingsMenu>,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
    ) {
        let mut is_open = true;
        ModalWindow::new(&mut is_open, &mut action_state, "Settings").show(egui.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                for tab in SettingsTab::iter() {
                    ui.selectable_value(&mut settings_menu.current_tab, tab, tab.to_string());
                }
            });
            match settings_menu.current_tab {
                SettingsTab::Video => VideoTab::new(&mut settings.video).show(ui),
                SettingsTab::Developer => DeveloperTab::new(&mut settings.developer).show(ui),
            };
            ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        apply_events.send(SettingsApplied);
                        commands.remove_resource::<SettingsMenu>();
                    }
                    if ui.button("Apply").clicked() {
                        apply_events.send(SettingsApplied);
                    }
                    if ui.button("Restore defaults").clicked() {
                        *settings = Settings::default();
                        apply_events.send(SettingsApplied);
                    }
                });
            });
        });

        if !is_open {
            commands.remove_resource::<SettingsMenu>();
        }
    }
}

#[derive(Default)]
pub(super) struct SettingsMenu {
    current_tab: SettingsTab,
}

#[derive(Default, Display, Clone, Copy, EnumIter, PartialEq)]
enum SettingsTab {
    #[default]
    Video,
    Developer,
}
