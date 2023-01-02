use bevy::prelude::*;
use bevy_renet::renet::RenetClient;
use bevy_trait_query::RegisterExt;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use tap::TapOptional;

use crate::core::{
    doll::DollPlayers,
    game_state::GameState,
    ground::Ground,
    network::network_event::client_event::ClientEvent,
    task::{Task, TaskActivation, TaskCancel, TaskList, TaskRequest, TaskRequestKind},
};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Task, Walk>()
            .add_system(Self::tasks_system.run_in_state(GameState::Family))
            .add_system(Self::activation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::cancellation_system.run_unless_resource_exists::<RenetClient>());
    }
}

impl MovementPlugin {
    fn tasks_system(mut ground: Query<&mut TaskList, (With<Ground>, Added<TaskList>)>) {
        if let Ok(mut task_list) = ground.get_single_mut() {
            task_list.tasks.push(TaskRequestKind::Walk);
        }
    }

    fn activation_system(
        mut commands: Commands,
        mut activation_events: EventReader<TaskActivation>,
    ) {
        for TaskActivation { entity, task } in activation_events.iter().copied() {
            if let TaskRequest::Walk(position) = task {
                commands.entity(entity).insert(Walk(position));
            }
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<ClientEvent<TaskCancel>>,
        dolls: Query<(Entity, &DollPlayers)>,
    ) {
        for ClientEvent { client_id, event } in cancel_events.iter().copied() {
            if let Some(entity) = dolls
                .iter()
                .find(|(.., players)| players.contains(&client_id))
                .map(|(entity, _)| entity)
                .tap_none(|| error!("no controlled entity for {event:?} for client {client_id}"))
            {
                if let TaskRequestKind::Walk = event.0 {
                    commands.entity(entity).remove::<Walk>();
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
pub(crate) struct Walk(pub(crate) Vec3);

impl Task for Walk {
    fn kind(&self) -> TaskRequestKind {
        TaskRequestKind::Walk
    }
}
