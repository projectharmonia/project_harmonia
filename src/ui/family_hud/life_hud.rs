use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Area, ImageButton, TextureId},
    EguiContexts,
};
use bevy_inspector_egui::egui::Frame;

use crate::core::{
    doll::ActiveDoll,
    family::FamilyMode,
    game_state::GameState,
    task::{Task, TaskCancel, TaskQueue, TaskRequestKind, TaskRequestRemove},
};

pub(super) struct LifeHudPlugin;

impl Plugin for LifeHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::active_tasks_system
                .in_set(OnUpdate(GameState::Family))
                .in_set(OnUpdate(FamilyMode::Life)),
        );
    }
}

impl LifeHudPlugin {
    fn active_tasks_system(
        mut egui: EguiContexts,
        mut cancel_events: EventWriter<TaskCancel>,
        mut remove_events: EventWriter<TaskRequestRemove>,
        tasks: Query<(&TaskQueue, Option<&dyn Task>), With<ActiveDoll>>,
    ) {
        const ICON_SIZE: f32 = 50.0;
        Area::new("Tasks")
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                let (task_queue, active_tasks) = tasks.single();
                // Show frame with window spacing, but without visuals.
                let queued_frame = Frame {
                    inner_margin: ui.spacing().window_margin,
                    rounding: ui.visuals().window_rounding,
                    ..Frame::none()
                };
                queued_frame.show(ui, |ui| {
                    for (id, task) in task_queue.iter() {
                        let button =
                            ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                        if ui
                            .add(button)
                            .on_hover_text(TaskRequestKind::from(task).to_string())
                            .clicked()
                        {
                            remove_events.send(TaskRequestRemove(id));
                        }
                    }
                });
                Frame::window(ui.style()).show(ui, |ui| {
                    let mut task_count = 0;
                    for task in active_tasks.into_iter().flatten() {
                        let button =
                            ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                        if ui
                            .add(button)
                            .on_hover_text(task.kind().to_string())
                            .clicked()
                        {
                            cancel_events.send(TaskCancel(task.kind()))
                        }
                        task_count += 1;
                    }

                    const MAX_ACTIVE_TASKS: u8 = 3;
                    let tasks_left = MAX_ACTIVE_TASKS - task_count;
                    let mut size = ui.spacing().window_margin.left_top();
                    size.x += ICON_SIZE + 2.0;
                    size.y += (ICON_SIZE + ui.spacing().item_spacing.y * 4.0) * tasks_left as f32;
                    ui.allocate_space(size);
                });
            });
    }
}
