mod controls_tab;
mod developer_tab;
mod input_events;
mod video_tab;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align, Layout},
    EguiContext,
};
use iyes_loopless::prelude::*;
use leafwing_input_manager::{prelude::ActionState, user_input::InputKind};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    modal_window::{ModalUiExt, ModalWindow},
    ui_action::UiAction,
};
use crate::core::{
    control_action::ControlAction,
    settings::{Settings, SettingsApply},
};
use controls_tab::ControlsTab;
use developer_tab::DeveloperTab;
use input_events::InputEvents;
use video_tab::VideoTab;

pub(super) struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::settings_menu_system.run_if_resource_exists::<SettingsMenu>())
            .add_system(
                Self::binding_window_system
                    .run_if_resource_exists::<ActiveBinding>()
                    .run_unless_resource_exists::<BindingConflict>(),
            )
            .add_system(
                Self::conflict_binding_window_system.run_if_resource_exists::<BindingConflict>(),
            );
    }
}

impl SettingsMenuPlugin {
    fn settings_menu_system(
        mut commands: Commands,
        mut apply_events: EventWriter<SettingsApply>,
        mut action_state: ResMut<ActionState<UiAction>>,
        mut settings_menu: ResMut<SettingsMenu>,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
    ) {
        let mut is_open = true;
        ModalWindow::new("Settings")
            .open(&mut is_open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    for tab in SettingsTab::iter() {
                        ui.selectable_value(&mut settings_menu.current_tab, tab, tab.to_string());
                    }
                });
                match settings_menu.current_tab {
                    SettingsTab::Video => VideoTab::new(&mut settings.video).show(ui),
                    SettingsTab::Controls => {
                        ControlsTab::new(&mut settings.controls).show(ui, &mut commands)
                    }
                    SettingsTab::Developer => DeveloperTab::new(&mut settings.developer).show(ui),
                };
                ui.with_layout(Layout::left_to_right(Align::Max), |ui| {
                    if ui.button("Ok").clicked() {
                        apply_events.send_default();
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                    if ui.button("Apply").clicked() {
                        apply_events.send_default();
                    }
                    ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
                        if ui.button("Restore defaults").clicked() {
                            *settings = Settings::default();
                            apply_events.send_default();
                        }
                    });
                });
            });

        if !is_open {
            commands.remove_resource::<SettingsMenu>();
        }
    }

    fn binding_window_system(
        mut commands: Commands,
        mut input_events: InputEvents,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
        mut action_state: ResMut<ActionState<UiAction>>,
        active_binding: Res<ActiveBinding>,
    ) {
        let mut open = true;
        ModalWindow::new(format!("Binding \"{}\"", active_binding.action))
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label("Press any key now or Esc to cancel");
                if let Some(input_kind) = input_events.input_kind() {
                    let conflict_action =
                        settings
                            .controls
                            .mappings
                            .iter()
                            .find_map(|(inputs, action)| {
                                if action != active_binding.action
                                    && inputs.contains(&input_kind.into())
                                {
                                    return Some(action);
                                }
                                None
                            });
                    if let Some(action) = conflict_action {
                        commands.insert_resource(BindingConflict { action, input_kind })
                    } else {
                        settings.controls.mappings.insert_at(
                            input_kind,
                            active_binding.action,
                            active_binding.index,
                        );
                        ui.close_modal();
                    }
                }
            });

        if !open {
            commands.remove_resource::<ActiveBinding>();
        }
    }

    fn conflict_binding_window_system(
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut settings: ResMut<Settings>,
        mut action_state: ResMut<ActionState<UiAction>>,
        active_binding: Res<ActiveBinding>,
        binding_conflict: Res<BindingConflict>,
    ) {
        let mut open = true;
        ModalWindow::new(format!("Binding \"{}\"", active_binding.action))
            .open(&mut open, &mut action_state)
            .show(egui.ctx_mut(), |ui| {
                ui.label(format!(
                    "Input \"{}\" is already used by \"{}\"",
                    binding_conflict.input_kind, binding_conflict.action
                ));
                ui.horizontal(|ui| {
                    if ui.button("Replace").clicked() {
                        settings
                            .controls
                            .mappings
                            .remove(binding_conflict.action, binding_conflict.input_kind);
                        settings.controls.mappings.insert_at(
                            binding_conflict.input_kind,
                            active_binding.action,
                            active_binding.index,
                        );
                        ui.close_modal();
                    }
                    if ui.button("Cancel").clicked() {
                        ui.close_modal();
                    }
                });
            });

        if !open {
            commands.remove_resource::<ActiveBinding>();
            commands.remove_resource::<BindingConflict>();
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
    Controls,
    Developer,
}

struct ActiveBinding {
    action: ControlAction,
    index: usize,
}

impl ActiveBinding {
    fn new(action: ControlAction, index: usize) -> Self {
        Self { action, index }
    }
}

struct BindingConflict {
    action: ControlAction,
    input_kind: InputKind,
}
