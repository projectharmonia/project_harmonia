mod buy_lot;
mod linked_task;
mod movement;

use std::{
    any,
    fmt::{self, Debug, Formatter},
};

use bevy::{
    ecs::entity::EntityMap,
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        TypeRegistryInternal,
    },
};
use bevy_replicon::prelude::*;
use bitflags::bitflags;
use leafwing_input_manager::common_conditions::action_just_pressed;
use serde::{
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use strum::{EnumVariantNames, IntoStaticStr, VariantNames};

use crate::core::{
    action::Action, actor::Actor, component_commands::ComponentCommandsExt, family::FamilyMode,
    game_state::GameState,
};
use buy_lot::BuyLotPlugin;
use linked_task::LinkedTaskPlugin;
use movement::MovementPlugin;

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((BuyLotPlugin, LinkedTaskPlugin, MovementPlugin)).replicate::<TaskState>()
            .add_mapped_client_reflect_event::<TaskRequest, TaskRequestSerializer, TaskRequestDeserializer>(SendPolicy::Unordered)
            .add_client_event::<TaskCancel>(SendPolicy::Unordered)
            .add_event::<TaskList>()
            .configure_set(
                Update,
                TaskListSet
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(in_state(GameState::Family))
                    .run_if(in_state(FamilyMode::Life)),
            )
            .add_systems(
                Update,
                (
                    Self::queue_system,
                    Self::activation_system,
                    Self::cancelation_system,
                )
                    .run_if(has_authority()),
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
                        .insert_reflect([event.task.into_reflect()]);
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
                .filter(|(_, &state)| state == TaskState::Active)
                .map(|(&groups, _)| groups)
                .reduce(|acc, groups| acc & groups)
                .unwrap_or_default();

            let mut iter = tasks.iter_many_mut(children);
            while let Some((groups, mut state)) = iter.fetch_next() {
                if *state == TaskState::Queued && !groups.intersects(current_groups) {
                    *state = TaskState::Active;
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
        for event in cancel_events.iter().map(|event| &event.event) {
            if let Ok(mut state) = tasks.get_mut(event.0) {
                match *state {
                    TaskState::Queued => commands.entity(event.0).despawn(),
                    TaskState::Active => *state = TaskState::Cancelled,
                    TaskState::Cancelled => (),
                }
            } else {
                error!("entity {:?} is not a task", event.0);
            }
        }
    }
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

#[derive(Clone, Component, Copy, Default, PartialEq, Reflect)]
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
pub(crate) trait Task: Reflect + Debug {
    fn name(&self) -> &str;
    fn groups(&self) -> TaskGroups {
        TaskGroups::default()
    }
}

/// An event of canceling the specified task.
///
/// Emitted by players.
#[derive(Debug, Deserialize, Event, Serialize)]
pub(crate) struct TaskCancel(pub(crate) Entity);

impl MapEventEntities for TaskCancel {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapError> {
        self.0 = entity_map.get(self.0).ok_or(MapError(self.0))?;
        Ok(())
    }
}

#[derive(Debug, Event)]
pub(crate) struct TaskRequest {
    pub(crate) entity: Entity,
    pub(crate) task: Box<dyn Task>,
}

impl MapEventEntities for TaskRequest {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapError> {
        self.entity = entity_map.get(self.entity).ok_or(MapError(self.entity))?;
        Ok(())
    }
}

#[derive(IntoStaticStr, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
enum TaskRequestField {
    Entity,
    Task,
}

struct TaskRequestSerializer<'a> {
    event: &'a TaskRequest,
    registry: &'a TypeRegistryInternal,
}

impl BuildEventSerializer<TaskRequest> for TaskRequestSerializer<'_> {
    type EventSerializer<'a> = TaskRequestSerializer<'a>;

    fn new<'a>(
        event: &'a TaskRequest,
        registry: &'a TypeRegistryInternal,
    ) -> Self::EventSerializer<'a> {
        Self::EventSerializer { event, registry }
    }
}

impl Serialize for TaskRequestSerializer<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct(
            any::type_name::<TaskRequest>(),
            TaskRequestField::VARIANTS.len(),
        )?;
        state.serialize_field(TaskRequestField::Entity.into(), &self.event.entity)?;
        state.serialize_field(
            TaskRequestField::Task.into(),
            &ReflectSerializer::new(self.event.task.as_reflect(), self.registry),
        )?;
        state.end()
    }
}

struct TaskRequestDeserializer<'a> {
    registry: &'a TypeRegistryInternal,
}

impl BuildEventDeserializer for TaskRequestDeserializer<'_> {
    type EventDeserializer<'a> = TaskRequestDeserializer<'a>;

    fn new(registry: &TypeRegistryInternal) -> Self::EventDeserializer<'_> {
        Self::EventDeserializer { registry }
    }
}

impl<'de> DeserializeSeed<'de> for TaskRequestDeserializer<'_> {
    type Value = TaskRequest;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct(
            any::type_name::<Self::Value>(),
            TaskRequestField::VARIANTS,
            self,
        )
    }
}

impl<'de> Visitor<'de> for TaskRequestDeserializer<'_> {
    type Value = TaskRequest;

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str(any::type_name::<Self::Value>())
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let entity = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(TaskRequestField::Entity as usize, &self))?;
        let reflect = seq
            .next_element_seed(UntypedReflectDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_length(TaskRequestField::Task as usize, &self))?;
        let type_name = reflect.type_name();
        let registration = self
            .registry
            .get(reflect.type_id())
            .ok_or_else(|| de::Error::custom(format!("{type_name} is not registered")))?;
        let reflect_task = registration
            .data::<ReflectTask>()
            .ok_or_else(|| de::Error::custom(format!("{type_name} doesn't have reflect(Task)")))?;
        let task = reflect_task.get_boxed(reflect).map_err(|reflect| {
            de::Error::custom(format!("{} is not a Task", reflect.type_name()))
        })?;

        Ok(TaskRequest { entity, task })
    }
}

#[cfg(test)]
mod tests {
    use serde_test::Token;

    use super::*;

    #[test]
    fn task_request_ser() {
        let mut registry = TypeRegistryInternal::new();
        registry.register::<DummyTask>();
        let task_request = TaskRequest {
            entity: Entity::PLACEHOLDER,
            task: Box::new(DummyTask),
        };
        let serializer = TaskRequestSerializer::new(&task_request, &registry);

        serde_test::assert_ser_tokens(
            &serializer,
            &[
                Token::Struct {
                    name: any::type_name::<TaskRequest>(),
                    len: TaskRequestField::VARIANTS.len(),
                },
                Token::Str(TaskRequestField::Entity.into()),
                Token::U64(task_request.entity.to_bits()),
                Token::Str(TaskRequestField::Task.into()),
                Token::Map { len: Some(1) },
                Token::Str(any::type_name::<DummyTask>()),
                Token::Struct {
                    name: "DummyTask",
                    len: 0,
                },
                Token::StructEnd,
                Token::MapEnd,
                Token::StructEnd,
            ],
        );
    }

    #[derive(Reflect, Debug)]
    struct DummyTask;

    impl Task for DummyTask {
        fn name(&self) -> &str {
            unimplemented!()
        }
    }
}
