mod movement;

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_renet::renet::RenetServer;
use derive_more::From;
use iyes_loopless::prelude::IntoConditionalSystem;
use serde::{Deserialize, Serialize};

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
        app.add_client_event::<Task>()
            .add_system(Self::picking_system.run_in_state(GameState::Family))
            .add_system(Self::queue_system.run_if_resource_exists::<RenetServer>());
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
        mut task_events: EventReader<ClientEvent<Task>>,
        mut dolls: Query<(&mut QueuedTasks, &DollPlayers)>,
    ) {
        for ClientEvent { client_id, event } in task_events.iter().copied() {
            for (mut tasks, players) in &mut dolls {
                if players.contains(&client_id) {
                    tasks.push(event);
                    break;
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, From)]
pub(crate) enum Task {
    Walk(Walk),
}

#[derive(Component)]
pub(crate) struct TaskList {
    pub(crate) position: Vec3,
    pub(crate) tasks: Vec<Task>,
}

impl TaskList {
    fn new(position: Vec3) -> Self {
        Self {
            position,
            tasks: Default::default(),
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub(crate) struct QueuedTasks(Vec<Task>);
