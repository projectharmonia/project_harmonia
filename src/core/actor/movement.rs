use bevy::prelude::*;
use bevy_mod_replication::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};
use tap::TapOptional;

use crate::core::{
    actor::Players,
    family::FamilyMode,
    game_state::GameState,
    ground::Ground,
    task::{Task, TaskActivation, TaskCancel, TaskList, TaskRequest, TaskRequestKind},
};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Task, Walk>()
            .add_system(
                Self::tasks_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (Self::activation_system, Self::cancellation_system).in_set(ServerSet::Authority),
            );
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
        actors: Query<(Entity, &Players)>,
    ) {
        for ClientEvent { client_id, event } in cancel_events.iter().copied() {
            if let Some(entity) = actors
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
