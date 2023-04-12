use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_replicon::prelude::*;
use bevy_trait_query::{queryable, One};
use leafwing_input_manager::common_conditions::action_just_pressed;
use serde::{Deserialize, Serialize};
use strum::EnumDiscriminants;

use super::{
    action::Action,
    actor::{movement::Walk, FirstName, Players},
    component_commands::ComponentCommandsExt,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    lot::BuyLot,
};

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<QueuedTask>()
            .add_client_event::<TaskRequest>()
            .add_client_event::<ActiveTaskCancel>()
            .add_client_event::<QueuedTaskCancel>()
            .add_system(
                Self::task_list_system
                    .run_if(action_just_pressed(Action::Confirm))
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::activation_system,
                    Self::queue_system,
                    Self::queued_cancelation_system,
                    Self::active_cancelation_system,
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

            commands.entity(hovered_entity).insert(TaskList);
        }
    }

    fn queue_system(
        mut commands: Commands,
        mut task_events: EventReader<FromClient<TaskRequest>>,
        mut actors: Query<(Entity, &Players)>,
    ) {
        for FromClient { client_id, event } in &mut task_events {
            if let Some((entity, _)) = actors
                .iter_mut()
                .find(|(_, players)| players.contains(client_id))
            {
                commands.entity(entity).with_children(|parent| {
                    let mut task_entity = parent.spawn((QueuedTask, Replication));
                    match event {
                        TaskRequest::Walk(walk) => task_entity.insert(*walk),
                        TaskRequest::BuyLot(buy) => task_entity.insert(*buy),
                    };
                });
            } else {
                error!("no controlled entity for {event:?} for client {client_id}");
            }
        }
    }

    fn activation_system(
        mut commands: Commands,
        queued_tasks: Query<(Entity, One<&dyn Task>), With<QueuedTask>>,
        actors: Query<(Entity, &Children), With<FirstName>>,
    ) {
        for (actor_entity, children) in &actors {
            if let Some((task_entity, task)) = queued_tasks.iter_many(children).next() {
                commands
                    .entity(actor_entity)
                    .insert_components(vec![task.clone_value()]);
                commands.entity(task_entity).despawn();
            }
        }
    }

    fn queued_cancelation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<QueuedTaskCancel>>,
        queued_tasks: Query<(), With<QueuedTask>>,
    ) {
        for FromClient { client_id, event } in &mut cancel_events {
            if queued_tasks.get(event.0).is_ok() {
                commands.entity(event.0).despawn();
            } else {
                error!("{event:?} from client {client_id} points to not a queued task");
            }
        }
    }

    fn active_cancelation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<ActiveTaskCancel>>,
        actors: Query<(Entity, &Players)>,
    ) {
        for FromClient { client_id, event } in &mut cancel_events {
            if let Some((entity, _)) = actors
                .iter()
                .find(|(_, players)| players.contains(client_id))
            {
                let mut entity = commands.entity(entity);
                match event.0 {
                    TaskRequestKind::Walk => entity.remove::<Walk>(),
                    TaskRequestKind::BuyLot => entity.remove::<BuyLot>(),
                };
            } else {
                error!("no controlled entity for {event:?} for client {client_id}");
            }
        }
    }
}

/// Marker that indicates that the entity contains list of possible tasks as children.
///
/// Added when clicking on objects.
#[derive(Component)]
pub(crate) struct TaskList;

/// Marker that indicates that the entity represents a queued task.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(super) struct QueuedTask;

/// Event with requested task to put in queue.
///
/// Emitted by players.
#[derive(Serialize, Deserialize, Debug, EnumDiscriminants)]
#[strum_discriminants(name(TaskRequestKind))]
#[strum_discriminants(derive(Serialize, Deserialize))]
pub(crate) enum TaskRequest {
    Walk(Walk),
    BuyLot(BuyLot),
}

/// An event of canceling an active task from the currently active player.
///
/// Emitted by players.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ActiveTaskCancel(pub(crate) TaskRequestKind);

/// An event of canceling a queued actor task.
///
/// Emitted by players.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct QueuedTaskCancel(pub(crate) Entity);

impl MapEntities for QueuedTaskCancel {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// A trait to mark component as task.
///
/// Will be counted as an ongoing task when exists on an actor.
#[queryable]
pub(crate) trait Task: Reflect {
    /// Task name to display.
    fn name(&self) -> &'static str;

    /// Converts itself to the request event.
    fn to_request(&self) -> TaskRequest;

    /// Returns the corresponding request discriminant.
    fn to_request_kind(&self) -> TaskRequestKind;
}
