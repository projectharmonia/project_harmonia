mod movement;

use bevy::{app::PluginGroupBuilder, prelude::*, reflect::FromReflect};
use bevy_renet::renet::RenetServer;
use bevy_trait_query::impl_trait_query;
use derive_more::From;
use iyes_loopless::prelude::IntoConditionalSystem;
use serde::{Deserialize, Serialize};
use strum::Display;
use tap::TapOptional;

use self::movement::MovementPlugin;

use super::{
    doll::DollPlayers,
    game_state::GameState,
    network::network_event::client_event::{ClientEvent, ClientEventAppExt},
    picking::ObjectPicked,
};
use movement::Walk;

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
            .add_client_event::<TaskKind>()
            .add_system(Self::picking_system.run_in_state(GameState::Family))
            .add_system(Self::queue_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::activation_system.run_if_resource_exists::<RenetServer>());
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
        mut task_events: EventReader<ClientEvent<TaskKind>>,
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
                    TaskKind::Walk(walk) => commands.entity(entity).insert(walk),
                };
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Display, From, FromReflect, Reflect, Serialize)]
pub(crate) enum TaskKind {
    Walk(Walk),
}

#[derive(Component)]
pub(crate) struct TaskList {
    pub(crate) position: Vec3,
    pub(crate) tasks: Vec<TaskKind>,
}

impl TaskList {
    fn new(position: Vec3) -> Self {
        Self {
            position,
            tasks: Default::default(),
        }
    }
}

impl_trait_query!(Task);

pub(crate) trait Task: Reflect {
    fn name(&self) -> &'static str;
}

#[derive(Clone, Component, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct QueuedTasks(Vec<TaskKind>);
