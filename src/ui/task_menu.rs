use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::{
    egui::{Align, Layout, Pos2},
    EguiContexts,
};

use crate::core::{
    actor::ActiveActor,
    family::FamilyMode,
    game_state::GameState,
    task::{ListedTask, TaskComponents, TaskList, TaskRequest},
};

pub(super) struct TaskMenuPlugin;

impl Plugin for TaskMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::menu_system
                .in_set(OnUpdate(GameState::Family))
                .in_set(OnUpdate(FamilyMode::Life)),
        );
    }
}

impl TaskMenuPlugin {
    fn menu_system(
        mut position: Local<Pos2>,
        mut commands: Commands,
        mut set: ParamSet<(&World, EguiContexts, EventWriter<TaskRequest>)>,
        task_components: Res<TaskComponents>,
        registry: Res<AppTypeRegistry>,
        windows: Query<&Window, With<PrimaryWindow>>,
        task_lists: Query<(Entity, &Name, Option<&Children>, Ref<TaskList>)>,
        tasks: Query<(Entity, &Name), With<ListedTask>>,
        active_actors: Query<Entity, With<ActiveActor>>,
    ) {
        let Ok((entity, name, children, task_list)) = task_lists.get_single() else {
            return;
        };

        if task_list.is_added() {
            // Recalculate window position.
            let primary_window = windows.single();
            let cursor_position = primary_window.cursor_position().unwrap_or_default();
            position.x = cursor_position.x;
            position.y = primary_window.height() - cursor_position.y;
        }

        let mut open = true;
        let mut task_entity = None;
        bevy_egui::egui::Window::new(name.as_str())
            .resizable(false)
            .collapsible(false)
            .fixed_pos(*position)
            .default_width(130.0)
            .open(&mut open)
            .show(set.p1().ctx_mut(), |ui| {
                ui.with_layout(Layout::top_down_justified(Align::Min), |ui| {
                    for (entity, name) in tasks.iter_many(children.into_iter().flatten()) {
                        if ui.button(&**name).clicked() {
                            task_entity = Some(entity);
                        }
                    }
                });
            });

        if let Some(task_entity) = task_entity {
            let task_entity = set.p0().entity(task_entity);
            let type_id = *task_components
                .iter()
                .find(|&&type_id| task_entity.contains_type_id(type_id))
                .expect("listed task should contain a registered component");
            let registry = registry.read();
            let registration = registry
                .get(type_id)
                .expect("all tasks should have registered TypeId");
            let type_name = registration.type_name();
            let reflect_component = registration
                .data::<ReflectComponent>()
                .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));
            let task = reflect_component
                .reflect(task_entity)
                .map(|task| task.clone_value())
                .unwrap_or_else(|| panic!("entity should have {type_name}"));
            set.p2().send(TaskRequest {
                entity: active_actors.single(),
                task,
            });

            open = false;
        }

        if !open {
            commands.entity(entity).remove::<TaskList>();
        }
    }
}
