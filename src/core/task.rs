use bevy::{math::Vec3Swizzles, prelude::*, reflect::FromReflect};
use bevy_renet::renet::RenetClient;
use bevy_trait_query::queryable;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants};
use tap::TapOptional;

use super::{
    doll::DollPlayers,
    game_state::GameState,
    network::network_event::client_event::{ClientEvent, ClientEventAppExt},
    picking::ObjectPicked,
};

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TaskQueue>()
            .register_type::<TaskRequest>()
            .register_type::<(u8, TaskRequest)>()
            .register_type::<Vec<(u8, TaskRequest)>>()
            .add_client_event::<TaskRequest>()
            .add_client_event::<TaskCancel>()
            .add_client_event::<TaskRequestRemove>()
            .add_event::<TaskActivation>()
            .add_system(Self::task_list_system.run_in_state(GameState::Family))
            .add_system(Self::queue_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::activation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::cancellation_system.run_unless_resource_exists::<RenetClient>());
    }
}

impl TaskPlugin {
    fn task_list_system(
        mut commands: Commands,
        mut pick_events: EventReader<ObjectPicked>,
        task_lists: Query<Entity, With<TaskList>>,
    ) {
        if let Some(event) = pick_events.iter().last() {
            if let Ok(entity) = task_lists.get_single() {
                commands.entity(entity).remove::<TaskList>();
            }

            commands
                .entity(event.entity)
                .insert(TaskList::new(event.position));
        }
    }

    fn queue_system(
        mut task_events: EventReader<ClientEvent<TaskRequest>>,
        mut dolls: Query<(&mut TaskQueue, &DollPlayers)>,
    ) {
        for ClientEvent { client_id, event } in task_events.iter().copied() {
            if let Some(mut task_queue) = dolls
                .iter_mut()
                .find(|(_, players)| players.contains(&client_id))
                .map(|(task_queue, _)| task_queue)
                .tap_none(|| error!("no controlled entity for {event:?} for client {client_id}"))
            {
                task_queue.push_task(event);
            }
        }
    }

    fn activation_system(
        mut activation_events: EventWriter<TaskActivation>,
        mut dolls: Query<(Entity, &mut TaskQueue)>,
    ) {
        for (entity, mut task_queue) in &mut dolls {
            if let Some(task) = task_queue.pop() {
                activation_events.send(TaskActivation { entity, task });
            }
        }
    }

    fn cancellation_system(
        mut remove_events: EventReader<ClientEvent<TaskRequestRemove>>,
        mut dolls: Query<(&mut TaskQueue, &DollPlayers)>,
    ) {
        for ClientEvent { client_id, event } in remove_events.iter().copied() {
            if let Some(mut task_queue) = dolls
                .iter_mut()
                .find(|(.., players)| players.contains(&client_id))
                .map(|(task_queue, _)| task_queue)
                .tap_none(|| error!("no controlled entity for {event:?} for client {client_id}"))
            {
                if let Some(index) = task_queue.queue.iter().position(|(id, _)| *id == event.0) {
                    task_queue.queue.remove(index);
                }
            }
        }
    }
}

/// Task request event.
#[derive(Clone, Copy, Debug, Deserialize, EnumDiscriminants, FromReflect, Reflect, Serialize)]
#[strum_discriminants(name(TaskRequestKind))]
#[strum_discriminants(derive(Display, Serialize, Deserialize))]
pub(crate) enum TaskRequest {
    Walk(Vec3),
    Buy(Vec2),
}

impl TaskRequest {
    #[must_use]
    pub(crate) fn new(task: TaskRequestKind, position: Vec3) -> Self {
        match task {
            TaskRequestKind::Walk => TaskRequest::Walk(position),
            TaskRequestKind::Buy => TaskRequest::Buy(position.xz()),
        }
    }
}

/// List of possible tasks for the entity.
///
/// The component is added after [`ObjectPicked`] event.
#[derive(Component)]
pub(crate) struct TaskList {
    /// The position on the entity at which the list was requested.
    pub(crate) position: Vec3,
    /// List of possible tasks for the assigned entity.
    pub(crate) tasks: Vec<TaskRequestKind>,
}

impl TaskList {
    #[must_use]
    fn new(position: Vec3) -> Self {
        Self {
            position,
            tasks: Default::default(),
        }
    }
}

#[queryable]
pub(crate) trait Task: Reflect {
    fn kind(&self) -> TaskRequestKind;
}

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

/// An event of removing a doll task from the queue.
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
