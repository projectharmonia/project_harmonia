mod movement;

use bevy::{app::PluginGroupBuilder, prelude::*, reflect::FromReflect};
use bevy_renet::renet::RenetServer;
use bevy_trait_query::impl_trait_query;
use iyes_loopless::prelude::IntoConditionalSystem;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumDiscriminants};
use tap::TapOptional;

use super::{
    doll::DollPlayers,
    game_state::GameState,
    network::network_event::client_event::{ClientEvent, ClientEventAppExt},
    picking::ObjectPicked,
};
use movement::{MovementPlugin, Walk};

pub(super) struct TaskPlugins;

impl PluginGroup for TaskPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(TaskPlugin).add(MovementPlugin);
    }
}

struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<QueuedTasks>()
            .add_client_event::<TaskRequest>()
            .add_client_event::<TaskCancel>()
            .add_system(Self::picking_system.run_in_state(GameState::Family))
            .add_system(Self::queue_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::activation_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::cancellation_system.run_if_resource_exists::<RenetServer>());
    }
}

impl TaskPlugin {
    fn picking_system(
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
        mut dolls: Query<(&mut QueuedTasks, &DollPlayers)>,
    ) {
        for ClientEvent { client_id, event } in task_events.iter().copied() {
            if let Some(mut tasks) = dolls
                .iter_mut()
                .find(|(_, players)| players.contains(&client_id))
                .map(|(task, _)| task)
                .tap_none(|| error!("no controlled entity for {event:?} for client {client_id}"))
            {
                tasks.push(event);
            }
        }
    }

    fn activation_system(mut commands: Commands, mut dolls: Query<(Entity, &mut QueuedTasks)>) {
        for (entity, mut tasks) in &mut dolls {
            if let Some(task) = tasks.pop() {
                match task {
                    TaskRequest::Walk(position) => commands.entity(entity).insert(Walk(position)),
                };
            }
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<ClientEvent<TaskCancel>>,
        mut dolls: Query<(Entity, &mut QueuedTasks, &DollPlayers)>,
    ) {
        for ClientEvent { client_id, event } in cancel_events.iter().copied() {
            if let Some((entity, mut tasks)) = dolls
                .iter_mut()
                .find(|(.., players)| players.contains(&client_id))
                .map(|(entity, task, _)| (entity, task))
                .tap_none(|| error!("no controlled entity for {event:?} for client {client_id}"))
            {
                if let Some(index) = tasks
                    .iter()
                    .map(TaskRequestKind::from)
                    .position(|task| task == event.0)
                {
                    tasks.swap_remove(index);
                } else {
                    match event.0 {
                        TaskRequestKind::Walk => commands.entity(entity).remove::<Walk>(),
                    };
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, EnumDiscriminants, FromReflect, Reflect, Serialize)]
#[strum_discriminants(name(TaskRequestKind))]
#[strum_discriminants(derive(Display, Serialize, Deserialize))]
pub(crate) enum TaskRequest {
    Walk(Vec3),
}

#[derive(Component)]
pub(crate) struct TaskList {
    /// The position on the entity at which the list was requested.
    pub(crate) position: Vec3,
    /// List of possible tasks for the assigned entity.
    pub(crate) tasks: Vec<TaskRequestKind>,
}

impl TaskList {
    fn new(position: Vec3) -> Self {
        Self {
            position,
            tasks: Default::default(),
        }
    }

    #[must_use]
    pub(crate) fn queue_task(&self, task: TaskRequestKind) -> TaskRequest {
        match task {
            TaskRequestKind::Walk => TaskRequest::Walk(self.position),
        }
    }
}

impl_trait_query!(Task);

pub(crate) trait Task: Reflect {
    fn kind(&self) -> TaskRequestKind;
}

#[derive(Clone, Component, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct QueuedTasks(Vec<TaskRequest>);

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub(crate) struct TaskCancel(pub(crate) TaskRequestKind);
