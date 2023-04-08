use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Area, Frame, ImageButton, TextureId},
    EguiContexts,
};

use crate::core::{
    actor::ActiveActor,
    family::{ActiveFamily, Actors, Budget, FamilyMode},
    game_state::GameState,
    task::{Task, TaskCancel, TaskQueue, TaskRequestKind, TaskRequestRemove},
};

pub(super) struct LifeHudPlugin;

impl Plugin for LifeHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                Self::active_tasks_system,
                Self::portrait_system,
                Self::family_members_system,
            )
                .in_set(OnUpdate(GameState::Family))
                .in_set(OnUpdate(FamilyMode::Life)),
        );
    }
}

const ICON_SIZE: f32 = 50.0;
const PORTRAIT_WIDTH: f32 = 150.0;

impl LifeHudPlugin {
    fn active_tasks_system(
        mut egui: EguiContexts,
        mut cancel_events: EventWriter<TaskCancel>,
        mut remove_events: EventWriter<TaskRequestRemove>,
        active_actors: Query<(&TaskQueue, Option<&dyn Task>), With<ActiveActor>>,
    ) {
        Area::new("Tasks")
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                let (task_queue, active_tasks) = active_actors.single();
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
                Frame::canvas(ui.style()).show(ui, |ui| {
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

                    ui.set_visible(false);
                    const MAX_ACTIVE_TASKS: u8 = 3;
                    for _ in task_count..MAX_ACTIVE_TASKS {
                        let button =
                            ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                        ui.add(button);
                    }
                });
            });
    }

    fn portrait_system(
        mut egui: EguiContexts,
        active_families: Query<&Budget, With<ActiveFamily>>,
    ) {
        let ctx = egui.ctx_mut();
        let button_padding = ctx.style().spacing.button_padding;
        let item_spacing = ctx.style().spacing.item_spacing;
        let left_offset = ICON_SIZE + button_padding.x + item_spacing.x;

        Area::new("Portrait")
            .anchor(Align2::LEFT_BOTTOM, (left_offset, 0.0))
            .show(ctx, |ui| {
                Frame::canvas(ui.style()).show(ui, |ui| {
                    let budget = active_families.single();
                    ui.label(budget.to_string() + " 💳");

                    ui.allocate_space((PORTRAIT_WIDTH, 0.0).into());
                });
            });
    }

    fn family_members_system(
        mut commands: Commands,
        mut egui: EguiContexts,
        active_families: Query<&Actors, With<ActiveFamily>>,
        active_actors: Query<Entity, With<ActiveActor>>,
    ) {
        let ctx = egui.ctx_mut();
        let button_padding = ctx.style().spacing.button_padding;
        let item_spacing = ctx.style().spacing.item_spacing;
        let left_offset = ICON_SIZE + PORTRAIT_WIDTH + button_padding.x + item_spacing.x;

        Area::new("Family members")
            .anchor(Align2::LEFT_BOTTOM, (left_offset, 0.0))
            .show(ctx, |ui| {
                Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let mut members_count = 0;
                        for &entity in active_families.single().iter() {
                            let button =
                                ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE))
                                    .selected(active_actors.get(entity).is_ok());
                            if ui.add(button).clicked() {
                                commands.entity(entity).insert(ActiveActor);
                                commands
                                    .entity(active_actors.single())
                                    .remove::<ActiveActor>();
                            }
                            members_count += 1;
                        }

                        ui.set_visible(false);
                        const MAX_FAMILY_SIZE: usize = 4;
                        for _ in members_count..MAX_FAMILY_SIZE {
                            let button =
                                ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                            ui.add(button);
                        }
                    })
                });
            });
    }
}
