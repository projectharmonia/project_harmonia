use std::{
    any,
    fmt::{self, Debug, Formatter},
};

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        TypeRegistryInternal,
    },
};
use bevy_replicon::prelude::*;
use bevy_trait_query::{queryable, One};
use bitflags::bitflags;
use leafwing_input_manager::common_conditions::action_just_pressed;
use serde::{
    de::{self, DeserializeSeed, SeqAccess, Visitor},
    ser::SerializeStruct,
    Deserialize, Deserializer, Serialize, Serializer,
};
use strum::{EnumVariantNames, IntoStaticStr, VariantNames};

use super::{
    action::Action, actor::Actor, component_commands::ComponentCommandsExt,
    cursor_hover::CursorHover, family::FamilyMode, game_state::GameState,
};

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<QueuedTask>()
            .add_mapped_client_reflect_event::<TaskRequest, TaskRequestSerializer, TaskRequestDeserializer>()
            .add_client_event::<ActiveTaskCancel>()
            .add_client_event::<QueuedTaskCancel>()
            .add_system(
                    Self::task_list_system.run_if(action_just_pressed(Action::Confirm))
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

            commands.entity(hovered_entity).insert(TaskList::default());
        }
    }

    fn queue_system(
        mut commands: Commands,
        mut task_events: ResMut<Events<FromClient<TaskRequest>>>,
        actors: Query<(), With<Actor>>,
    ) {
        for event in task_events.drain().map(|event| event.event) {
            if actors.get(event.entity).is_ok() {
                commands.entity(event.entity).with_children(|parent| {
                    parent
                        .spawn((QueuedTask, Replication))
                        .insert_reflect([event.task.into_reflect()]);
                });
            } else {
                error!("entity {:?} is not an actor", event.entity);
            }
        }
    }

    fn activation_system(
        mut commands: Commands,
        queued_tasks: Query<(Entity, One<&dyn Task>), With<QueuedTask>>,
        actors: Query<(Entity, &Children, Option<&dyn Task>), With<Actor>>,
    ) {
        for (actor_entity, children, tasks) in &actors {
            let current_groups = tasks
                .iter()
                .flatten()
                .map(|task| task.groups())
                .reduce(|acc, e| acc & e)
                .unwrap_or_default();

            for (task_entity, task) in queued_tasks
                .iter_many(children)
                .filter(|(_, task)| !task.groups().intersects(current_groups))
            {
                commands
                    .entity(actor_entity)
                    .insert_reflect([task.clone_value()]);
                commands.entity(task_entity).despawn();
            }
        }
    }

    fn queued_cancelation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<QueuedTaskCancel>>,
        queued_tasks: Query<(), With<QueuedTask>>,
    ) {
        for event in cancel_events.iter().map(|event| &event.event) {
            if queued_tasks.get(event.0).is_ok() {
                commands.entity(event.0).despawn();
            } else {
                error!("entity {:?} is not a task", event.0);
            }
        }
    }

    fn active_cancelation_system(
        mut commands: Commands,
        mut cancel_events: ResMut<Events<FromClient<ActiveTaskCancel>>>,
        registry: Res<AppTypeRegistry>,
    ) {
        let registry = registry.read();
        for event in cancel_events.drain().map(|event| event.event) {
            let Some(registration) = registry.get_with_name(&event.task_name) else {
                error!("{:?} is not registered", event.task_name);
                continue;
            };

            if registration.data::<ReflectTask>().is_some() {
                commands
                    .entity(event.entity)
                    .remove_by_name(event.task_name);
            } else {
                error!("{:?} doesn't have reflect(Task)", &event.task_name);
            }
        }
    }
}

/// Contains list of possible tasks.
///
/// Added when clicking on objects.
#[derive(Component, Default, Deref, DerefMut)]
pub(crate) struct TaskList(Vec<Box<dyn Task>>);

/// Marker that indicates that the entity represents a queued task.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(super) struct QueuedTask;

/// An event of canceling an active task from the currently active player.
///
/// Emitted by players.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ActiveTaskCancel {
    pub(crate) entity: Entity,
    pub(crate) task_name: String,
}

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
/// Will be counted as a queued task when exists on actor's children with component [`QueuedTask`].
#[queryable]
#[reflect_trait]
pub(crate) trait Task: Reflect + Debug {
    /// Task name to display.
    fn name(&self) -> &str;

    /// Returns task constraints.
    fn groups(&self) -> TaskGroups {
        TaskGroups::default()
    }
}

bitflags! {
    #[derive(Default)]
    pub(crate) struct TaskGroups: u8 {
        const LEFT_HAND = 0b00000001;
        const RIGHT_HAND = 0b00000010;
        const BOTH_HANDS = Self::LEFT_HAND.bits() | Self::RIGHT_HAND.bits();
        const LEGS = 0b00000100;
    }
}

#[derive(Debug)]
pub(crate) struct TaskRequest {
    pub(crate) entity: Entity,
    pub(crate) task: Box<dyn Task>,
}

impl MapEntities for TaskRequest {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.entity = entity_map.get(self.entity)?;
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
    registry: &'a TypeRegistryInternal,
    event: &'a TaskRequest,
}

impl BuildEventSerializer<TaskRequest> for TaskRequestSerializer<'_> {
    type EventSerializer<'a> = TaskRequestSerializer<'a>;

    fn new<'a>(
        registry: &'a TypeRegistryInternal,
        event: &'a TaskRequest,
    ) -> Self::EventSerializer<'a> {
        Self::EventSerializer { registry, event }
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
            de::Error::custom(format!("unable to cast {} to Task", reflect.type_name()))
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
        let serializer = TaskRequestSerializer::new(&registry, &task_request);

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
        fn name(&self) -> &'static str {
            "Dummy"
        }
    }
}
