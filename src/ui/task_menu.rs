use bevy::prelude::*;
use bevy_egui::{
    egui::{Pos2, Window},
    EguiContext,
};
use bevy_inspector_egui::egui::{Align, Layout};
use iyes_loopless::prelude::*;

use crate::core::{
    family::FamilyMode,
    game_state::GameState,
    task::{TaskList, TaskRequest},
};

pub(super) struct TaskMenuPlugin;

impl Plugin for TaskMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::menu_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Life),
        );
    }
}

impl TaskMenuPlugin {
    fn menu_system(
        mut position: Local<Pos2>,
        mut commands: Commands,
        mut egui: ResMut<EguiContext>,
        mut task_events: EventWriter<TaskRequest>,
        windows: Res<Windows>,
        task_lists: Query<(Entity, &Name, &TaskList, ChangeTrackers<TaskList>)>,
    ) {
        if let Ok((entity, name, task_list, task_list_changes)) = task_lists.get_single() {
            if task_list_changes.is_added() {
                // Recalculate window position.
                let primary_window = windows
                    .get_primary()
                    .expect("primary window should exist for UI");
                let cursor_position = primary_window.cursor_position().unwrap_or_default();
                position.x = cursor_position.x;
                position.y = primary_window.height() - cursor_position.y;
            }

            let mut open = true;
            Window::new(name.as_str())
                .resizable(false)
                .collapsible(false)
                .fixed_pos(*position)
                .default_width(130.0)
                .open(&mut open)
                .show(egui.ctx_mut(), |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                        for &task in &task_list.tasks {
                            if ui.button(task.to_string()).clicked() {
                                task_events.send(TaskRequest::new(task, task_list.position));
                                commands.entity(entity).remove::<TaskList>();
                            }
                        }
                    });
                });

            if !open {
                commands.entity(entity).remove::<TaskList>();
            }
        }
    }
}
