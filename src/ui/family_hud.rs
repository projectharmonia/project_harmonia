use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Area, ImageButton, TextureId},
    EguiContext,
};
use bevy_inspector_egui::egui::Frame;
use iyes_loopless::prelude::*;

use crate::core::{doll::ActiveDoll, game_state::GameState, task::QueuedTasks};

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::active_tasks_system.run_in_state(GameState::Family));
    }
}

impl FamilyHudPlugin {
    fn active_tasks_system(
        mut egui: ResMut<EguiContext>,
        queued_tasks: Query<&QueuedTasks, With<ActiveDoll>>,
    ) {
        const ICON_SIZE: f32 = 50.0;
        Area::new("Tasks")
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                // Show frame with window spacing, but without visuals.
                let queued_frame = Frame {
                    inner_margin: ui.spacing().window_margin,
                    rounding: ui.visuals().window_rounding,
                    ..Frame::none()
                };
                queued_frame.show(ui, |ui| {
                    if let Ok(tasks) = queued_tasks.get_single() {
                        for &task in tasks.iter() {
                            ui.add(ImageButton::new(
                                TextureId::Managed(0),
                                (ICON_SIZE, ICON_SIZE),
                            ))
                            .on_hover_text(task.to_string());
                        }
                    }
                });
                Frame::window(ui.style()).show(ui, |ui| {
                    const ACTIVE_TASKS: u8 = 3;
                    const ACTIVE_TASKS_HEIGHT: f32 = ICON_SIZE * ACTIVE_TASKS as f32;
                    let mut size = ui.spacing().window_margin.left_top();
                    size.x += ICON_SIZE + 2.0;
                    size.y += ACTIVE_TASKS_HEIGHT;

                    // TODO: disaply queued tasks.
                    ui.allocate_space(size);
                });
            });
    }
}
