mod developer_tab;
mod video_tab;

use bevy::prelude::*;
use bevy_egui::egui::Layout;
use bevy_egui::EguiContext;
use bevy_inspector_egui::egui::Align;
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
        app.init_resource::<SettingsMenu>()
            .add_system(Self::settings_menu_system.run_if(is_settings_open));
    }
}

impl SettingsMenuPlugin {
    fn settings_menu_system(
        mut apply_events: EventWriter<SettingsApplied>,
        action_state: Res<ActionState<UiAction>>,
        mut settings_menu: ResMut<SettingsMenu>,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
    ) {
        let SettingsMenu {
            ref mut is_open,
            ref mut current_tab,
        } = *settings_menu;

        ModalWindow::new(is_open, &action_state, "Settings").show(egui.ctx_mut(), |ui, is_open| {
            ui.horizontal(|ui| {
                for tab in SettingsTab::iter() {
                    ui.selectable_value(current_tab, tab, tab.to_string());
                }
            });
            match current_tab {
                SettingsTab::Video => VideoTab::new(&mut settings.video).show(ui),
                SettingsTab::Developer => DeveloperTab::new(&mut settings.developer).show(ui),
            };
            ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Ok").clicked() {
                        apply_events.send(SettingsApplied);
                        *is_open = false;
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
    }
}

fn is_settings_open(settings_menu: ResMut<SettingsMenu>) -> bool {
    settings_menu.is_open
}

#[derive(Default)]
pub(super) struct SettingsMenu {
    pub(super) is_open: bool,
    current_tab: SettingsTab,
}

#[derive(Default, Display, Clone, Copy, EnumIter, PartialEq)]
enum SettingsTab {
    #[default]
    Video,
    Developer,
}
