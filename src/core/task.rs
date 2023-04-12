use bevy::{math::Vec3Swizzles, prelude::*, reflect::FromReflect};
use bevy_replicon::prelude::*;
use bevy_trait_query::queryable;
use leafwing_input_manager::common_conditions::action_just_pressed;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants};

use super::{
    action::Action, actor::Players, cursor_hover::CursorHover, family::FamilyMode,
    game_state::GameState,
};

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<TaskQueue>()
            .register_type::<TaskRequest>()
            .register_type::<(u8, TaskRequest)>()
            .register_type::<Vec<(u8, TaskRequest)>>()
            .add_client_event::<TaskRequest>()
            .add_client_event::<TaskCancel>()
            .add_client_event::<TaskRequestRemove>()
            .add_event::<TaskActivation>()
            .add_system(
                Self::task_list_system
                    .run_if(action_just_pressed(Action::Confirm))
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::queue_system,
                    Self::activation_system,
                    Self::cancellation_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl TaskPlugin {
    fn task_list_system(
        mut commands: Commands,
        hovered: Query<Entity, With<CursorHover>>,
        task_lists: Query<Entity, With<TaskList>>,
    ) {
        if let Ok(hovered_entity) = hovered.get_single() {
            if let Ok(previous_entity) = task_lists.get_single() {
                commands.entity(previous_entity).remove::<TaskList>();
            }

            commands.entity(hovered_entity).insert(TaskList::default());
        }
    }

    fn queue_system(
        mut task_events: EventReader<FromClient<TaskRequest>>,
        mut actors: Query<(&mut TaskQueue, &Players)>,
    ) {
        for FromClient { client_id, event } in task_events.iter().copied() {
            if let Some((mut task_queue, _)) = actors
                .iter_mut()
                .find(|(_, players)| players.contains(&client_id))
            {
                task_queue.push_task(event);
            } else {
                error!("no controlled entity for {event:?} for client {client_id}");
            }
        }
    }

    fn activation_system(
        mut activation_events: EventWriter<TaskActivation>,
        mut actors: Query<(Entity, &mut TaskQueue)>,
    ) {
        for (entity, mut task_queue) in &mut actors {
            if let Some(task) = task_queue.pop() {
                activation_events.send(TaskActivation { entity, task });
            }
        }
    }

    fn cancellation_system(
        mut remove_events: EventReader<FromClient<TaskRequestRemove>>,
        mut actors: Query<(&mut TaskQueue, &Players)>,
    ) {
        for FromClient { client_id, event } in remove_events.iter().copied() {
            if let Some((mut task_queue, _)) = actors
                .iter_mut()
                .find(|(_, players)| players.contains(&client_id))
            {
                if let Some(index) = task_queue.queue.iter().position(|(id, _)| *id == event.0) {
                    task_queue.queue.remove(index);
                }
            } else {
                error!("no controlled entity for {event:?} for client {client_id}");
            }
        }
    }
}

/// List of possible tasks for the entity.
///
/// The component is added after clicking on object.
#[derive(Component, Default)]
pub(crate) struct TaskList {
    /// List of possible tasks for the assigned entity.
    ///
    /// Discriminants of [`TaskRequest`]
    pub(crate) tasks: Vec<TaskRequestKind>,
}

/// Event with requested task and it's data.
#[derive(Clone, Copy, Debug, Deserialize, EnumDiscriminants, FromReflect, Reflect, Serialize)]
#[strum_discriminants(name(TaskRequestKind))]
#[strum_discriminants(derive(Display, Serialize, Deserialize))]
pub(crate) enum TaskRequest {
    Walk(Vec3),
    Buy(Vec2),
}

impl TaskRequest {
    /// Creates a new task from the discriminant.
    #[must_use]
    pub(crate) fn new(task: TaskRequestKind, position: Vec3) -> Self {
        match task {
            TaskRequestKind::Walk => TaskRequest::Walk(position),
            TaskRequestKind::Buy => TaskRequest::Buy(position.xz()),
        }
    }
}

/// A trait to mark component as task.
///
/// Will be counted as an ongoing task when exists on an actor.
#[queryable]
pub(crate) trait Task: Reflect {
    fn kind(&self) -> TaskRequestKind;
}

/// List of pending tasks for an actor.
#[derive(Clone, Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct TaskQueue {
    queue: Vec<(u8, TaskRequest)>,
    next_id: u8,
}

impl TaskQueue {
    fn push_task(&mut self, task: TaskRequest) {
        self.queue.push((self.next_id, task));
        self.next_id += 1;
    }

    fn pop(&mut self) -> Option<TaskRequest> {
        self.queue.pop().map(|(_, task)| task)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (u8, TaskRequest)> + '_ {
        self.queue.iter().copied()
    }
}

/// An event of removing the active task from the player
///
/// Emitted by players.
#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub(crate) struct TaskCancel(pub(crate) TaskRequestKind);

/// An event of removing a actor task from the queue.
///
/// Emitted by players.
#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub(crate) struct TaskRequestRemove(pub(crate) u8);

/// Task activation event.
///
/// Emitted only on server to react on event activation in multiple systems.
#[derive(Clone, Copy)]
pub(crate) struct TaskActivation {
    pub(crate) entity: Entity,
    pub(crate) task: TaskRequest,
}
