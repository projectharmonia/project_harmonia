use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Area, ImageButton, TextureId},
    EguiContext,
};
use bevy_inspector_egui::egui::Frame;
use iyes_loopless::prelude::*;

use crate::core::{
    doll::ActiveDoll,
    game_state::GameState,
    task::{QueuedTasks, Task, TaskCancel, TaskRequestKind},
};

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::active_tasks_system.run_in_state(GameState::Family));
    }
}

impl FamilyHudPlugin {
    fn active_tasks_system(
        mut egui: ResMut<EguiContext>,
        mut cancel_events: EventWriter<TaskCancel>,
        tasks: Query<(&QueuedTasks, Option<&dyn Task>), With<ActiveDoll>>,
    ) {
        const ICON_SIZE: f32 = 50.0;
        Area::new("Tasks")
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                let (queued_tasks, active_tasks) = tasks.single();
                // Show frame with window spacing, but without visuals.
                let queued_frame = Frame {
                    inner_margin: ui.spacing().window_margin,
                    rounding: ui.visuals().window_rounding,
                    ..Frame::none()
                };
                queued_frame.show(ui, |ui| {
                    for task in queued_tasks.iter().map(TaskRequestKind::from) {
                        let button =
                            ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                        if ui.add(button).on_hover_text(task.to_string()).clicked() {
                            cancel_events.send(TaskCancel(task));
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
