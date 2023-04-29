use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Area, Frame, Grid, ImageButton, ProgressBar, RichText, TextureId},
    EguiContexts,
};
use bevy_trait_query::One;
use derive_more::Display;
use strum::{EnumIter, IntoEnumIterator};

use crate::core::{
    actor::{needs::Need, ActiveActor},
    family::{ActiveFamily, Budget, FamilyActors, FamilyMode},
    game_state::GameState,
    preview::{Preview, PreviewKind, Previews},
    task::{ActiveTaskCancel, QueuedTaskCancel, Task},
};

pub(super) struct LifeHudPlugin;

impl Plugin for LifeHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<InfoTab>().add_systems(
            (
                Self::active_tasks_system,
                Self::portrait_system,
                Self::family_members_system,
                Self::actor_info_system,
                Self::needs_system.in_set(OnUpdate(InfoTab::Needs)),
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
        mut active_cancel_events: EventWriter<ActiveTaskCancel>,
        mut queued_cancel_events: EventWriter<QueuedTaskCancel>,
        active_actors: Query<(Entity, Option<&Children>, Option<&dyn Task>), With<ActiveActor>>,
        queued_tasks: Query<(Entity, One<&dyn Task>)>,
    ) {
        Area::new("Tasks")
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                let (actor_entity, children, active_tasks) = active_actors.single();
                // Show frame with window spacing, but without visuals.
                let queued_frame = Frame {
                    inner_margin: ui.spacing().window_margin,
                    rounding: ui.visuals().window_rounding,
                    ..Frame::none()
                };
                queued_frame.show(ui, |ui| {
                    for (task_entity, task) in
                        queued_tasks.iter_many(children.iter().flat_map(|&children| children))
                    {
                        let button =
                            ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                        if ui.add(button).on_hover_text(task.name()).clicked() {
                            queued_cancel_events.send(QueuedTaskCancel(task_entity));
                        }
                    }
                });
                Frame::canvas(ui.style()).show(ui, |ui| {
                    let mut task_count = 0;
                    for task in active_tasks.into_iter().flatten() {
                        let button =
                            ImageButton::new(TextureId::Managed(0), (ICON_SIZE, ICON_SIZE));
                        if ui.add(button).on_hover_text(task.name()).clicked() {
                            active_cancel_events.send(ActiveTaskCancel {
                                entity: actor_entity,
                                task_name: task.type_name().to_string(),
                            });
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
        mut previews: ResMut<Previews>,
        active_families: Query<&FamilyActors, With<ActiveFamily>>,
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
                            let texture_id = previews.get(Preview {
                                kind: PreviewKind::Actor(entity),
                                size: ICON_SIZE as u32,
                            });
                            let button = ImageButton::new(texture_id, (ICON_SIZE, ICON_SIZE))
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

    fn actor_info_system(
        mut egui: EguiContexts,
        info_tab: Res<State<InfoTab>>,
        mut next_info_tab: ResMut<NextState<InfoTab>>,
    ) {
        Area::new("Actor info")
            .anchor(Align2::RIGHT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                Frame::canvas(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for tab in InfoTab::iter().skip(1) {
                            if ui
                                .selectable_label(
                                    info_tab.0 == tab,
                                    RichText::new(tab.glyph()).size(22.0),
                                )
                                .on_hover_text(tab.to_string())
                                .clicked()
                            {
                                if info_tab.0 == tab {
                                    next_info_tab.set(InfoTab::None);
                                } else {
                                    next_info_tab.set(tab);
                                }
                            }
                        }
                    });
                });
            });
    }

    fn needs_system(mut egui: EguiContexts, actors: Query<&dyn Need, With<ActiveActor>>) {
        Area::new("Needs")
            .anchor(Align2::RIGHT_BOTTOM, (0.0, -30.0))
            .show(egui.ctx_mut(), |ui| {
                Frame::canvas(ui.style()).inner_margin(5.0).show(ui, |ui| {
                    Grid::new("Needs grid")
                        .striped(true)
                        .num_columns(2)
                        .show(ui, |ui| {
                            for (index, need) in actors.single().iter().enumerate() {
                                let progress = ProgressBar::new(need.value() / 100.0)
                                    .text(RichText::new(need.glyph()).size(18.0))
                                    .desired_width(150.0);
                                ui.add(progress);
                                if (index + 1) % 2 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
            });
    }
}

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter, Default)]
enum InfoTab {
    #[default]
    None,
    Skills,
    Needs,
}

impl InfoTab {
    fn glyph(self) -> &'static str {
        match self {
            InfoTab::None => panic!("glyph shouldn't be requested without a tab"),
            InfoTab::Needs => "🗠", // 📐
            InfoTab::Skills => "💡",
        }
    }
}
