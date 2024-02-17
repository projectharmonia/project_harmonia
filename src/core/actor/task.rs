mod buy_lot;
mod friendly;
mod linked_task;
mod move_here;

use std::{fmt::Debug, io::Cursor};

use anyhow::{anyhow, Context, Result};
use bevy::{
    ecs::reflect::ReflectCommandExt,
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        TypeRegistry,
    },
};
use bevy_replicon::prelude::*;
use bincode::{DefaultOptions, Options};
use bitflags::bitflags;
use leafwing_input_manager::common_conditions::action_just_pressed;
use serde::{de::DeserializeSeed, Deserialize, Serialize};

use crate::core::{
    action::Action, actor::Actor, animation_state::AnimationState, family::FamilyMode,
    game_state::GameState, navigation::Navigation,
};
use buy_lot::BuyLotPlugin;
use friendly::FriendlyPlugins;
use linked_task::LinkedTaskPlugin;
use move_here::MoveHerePlugin;

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BuyLotPlugin,
            FriendlyPlugins,
            LinkedTaskPlugin,
            MoveHerePlugin,
        ))
        .register_type::<TaskState>()
        .replicate::<TaskState>()
        .add_client_event::<TaskCancel>(EventType::Unordered)
        .add_client_event_with::<TaskRequest, _, _>(
            EventType::Unordered,
            Self::sending_task_system,
            Self::receiving_task_system,
        )
        .add_event::<TaskList>()
        .configure_sets(
            Update,
            TaskListSet
                .run_if(action_just_pressed(Action::Confirm))
                .run_if(in_state(GameState::Family))
                .run_if(in_state(FamilyMode::Life)),
        )
        .add_systems(
            PreUpdate,
            (Self::queue_system, Self::cancelation_system)
                .after(ClientSet::Receive)
                .run_if(has_authority()),
        )
        .add_systems(
            PostUpdate,
            (Self::cleanup_system, Self::activation_system).run_if(has_authority()),
        );
    }
}

impl TaskPlugin {
    fn queue_system(
        mut commands: Commands,
        mut task_events: ResMut<Events<FromClient<TaskRequest>>>,
        actors: Query<(), With<Actor>>,
    ) {
        for event in task_events.drain().map(|event| event.event) {
            if actors.get(event.entity).is_ok() {
                commands.entity(event.entity).with_children(|parent| {
                    parent
                        .spawn(TaskBundle::new(&*event.task))
                        .insert_reflect(event.task.into_reflect());
                });
            } else {
                error!("entity {:?} is not an actor", event.entity);
            }
        }
    }

    fn activation_system(
        mut tasks: Query<(&TaskGroups, &mut TaskState)>,
        actors: Query<&Children, With<Actor>>,
    ) {
        for children in &actors {
            let current_groups = tasks
                .iter_many(children)
                .filter(|(_, &task_state)| task_state != TaskState::Queued)
                .map(|(&groups, _)| groups)
                .reduce(|acc, groups| acc & groups)
                .unwrap_or_default();

            let mut iter = tasks.iter_many_mut(children);
            while let Some((groups, mut task_state)) = iter.fetch_next() {
                if *task_state == TaskState::Queued && !groups.intersects(current_groups) {
                    *task_state = TaskState::Active;
                    break;
                }
            }
        }
    }

    fn cancelation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<TaskCancel>>,
        mut tasks: Query<&mut TaskState>,
    ) {
        for event in cancel_events.read().map(|event| &event.event) {
            if let Ok(mut task_state) = tasks.get_mut(event.0) {
                match *task_state {
                    TaskState::Queued => commands.entity(event.0).despawn(),
                    TaskState::Active => *task_state = TaskState::Cancelled,
                    TaskState::Cancelled => (),
                }
            } else {
                error!("entity {:?} is not a task", event.0);
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        tasks: Query<(Entity, &Parent, &TaskGroups, &TaskState), Changed<TaskState>>,
        mut actors: Query<&mut AnimationState>,
    ) {
        for (entity, parent, groups, &task_state) in &tasks {
            if task_state == TaskState::Cancelled {
                if groups.contains(TaskGroups::LEGS) {
                    commands.entity(**parent).remove::<Navigation>();
                }

                let mut animation_state = actors
                    .get_mut(**parent)
                    .expect("actor should have animaition state");
                animation_state.stop();

                commands.entity(entity).despawn();
            }
        }
    }

    fn sending_task_system(
        mut task_events: EventReader<TaskRequest>,
        mut client: ResMut<RenetClient>,
        channel: Res<ClientEventChannel<TaskRequest>>,
        registry: Res<AppTypeRegistry>,
    ) {
        let registry = registry.read();
        for event in task_events.read() {
            let message = serialize_task_request(event, &registry)
                .expect("client event should be serializable");

            client.send_message(*channel, message);
        }
    }

    fn receiving_task_system(
        mut task_events: EventWriter<FromClient<TaskRequest>>,
        mut server: ResMut<RenetServer>,
        channel: Res<ServerEventChannel<TaskRequest>>,
        registry: Res<AppTypeRegistry>,
        entity_map: Res<ServerEntityMap>,
    ) {
        let registry = registry.read();
        for client_id in server.clients_id() {
            while let Some(message) = server.receive_message(client_id, *channel) {
                match deserialize_task_request(&message, &registry) {
                    Ok(mut event) => {
                        event.map_entities(&mut EventMapper(entity_map.to_server()));
                        task_events.send(FromClient { client_id, event });
                    }
                    Err(e) => {
                        error!("unable to deserialize event from client {client_id}: {e}")
                    }
                }
            }
        }
    }
}

fn serialize_task_request(
    event: &TaskRequest,
    registry: &TypeRegistry,
) -> bincode::Result<Vec<u8>> {
    let mut message = Vec::new();
    let serializer = ReflectSerializer::new(event.task.as_reflect(), registry);
    DefaultOptions::new().serialize_into(&mut message, &event.entity)?;
    DefaultOptions::new().serialize_into(&mut message, &serializer)?;

    Ok(message)
}

fn deserialize_task_request(message: &[u8], registry: &TypeRegistry) -> Result<TaskRequest> {
    let mut cursor = Cursor::new(message);
    let entity = DefaultOptions::new().deserialize_from(&mut cursor)?;
    let mut deserializer = bincode::Deserializer::with_reader(&mut cursor, DefaultOptions::new());
    let reflect = UntypedReflectDeserializer::new(registry).deserialize(&mut deserializer)?;
    let type_info = reflect.get_represented_type_info().unwrap();
    let type_path = type_info.type_path();
    let registration = registry
        .get(type_info.type_id())
        .with_context(|| format!("{type_path} is not registered"))?;
    let reflect_task = registration
        .data::<ReflectTask>()
        .with_context(|| format!("{type_path} doesn't have reflect(Task)"))?;
    let task = reflect_task
        .get_boxed(reflect)
        .map_err(|_| anyhow!("{type_path} is not a Task"))?;

    Ok(TaskRequest { entity, task })
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) struct TaskListSet;

// Contains a task that should be listed.
///
/// Emitted when clicking on objects.
#[derive(Event)]
pub(crate) struct TaskList(pub(crate) Box<dyn Task>);

impl<T: Task> From<T> for TaskList {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

#[derive(Bundle)]
struct TaskBundle {
    name: Name,
    groups: TaskGroups,
    state: TaskState,
    parent_sync: ParentSync,
    replication: Replication,
}

impl TaskBundle {
    fn new(task: &dyn Task) -> Self {
        Self {
            name: Name::new(task.name().to_string()),
            groups: task.groups(),
            state: Default::default(),
            parent_sync: Default::default(),
            replication: Replication,
        }
    }
}

#[derive(Clone, Component, Copy, Default, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(crate) enum TaskState {
    #[default]
    Queued,
    Active,
    Cancelled,
}

bitflags! {
    #[derive(Default, Component, Clone, Copy)]
    pub(crate) struct TaskGroups: u8 {
        const LEFT_HAND = 0b00000001;
        const RIGHT_HAND = 0b00000010;
        const BOTH_HANDS = Self::LEFT_HAND.bits() | Self::RIGHT_HAND.bits();
        const LEGS = 0b00000100;
    }
}

#[reflect_trait]
pub(crate) trait Task: Reflect {
    fn name(&self) -> &str;
    fn groups(&self) -> TaskGroups {
        TaskGroups::default()
    }
}

/// An event of canceling the specified task.
///
/// Emitted by players.
#[derive(Deserialize, Event, Serialize)]
pub(crate) struct TaskCancel(pub(crate) Entity);

impl MapNetworkEntities for TaskCancel {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.0 = mapper.map(self.0);
    }
}

#[derive(Event)]
pub(crate) struct TaskRequest {
    pub(crate) entity: Entity,
    pub(crate) task: Box<dyn Task>,
}

impl MapNetworkEntities for TaskRequest {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.entity = mapper.map(self.entity);
    }
}
