use std::{
    any::{self, TypeId},
    fmt::{self, Formatter},
};

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        GetTypeRegistration, TypeRegistryInternal,
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

use super::{
    action::Action, actor::Actor, component_commands::ComponentCommandsExt,
    cursor_hover::CursorHover, family::FamilyMode, game_state::GameState,
};

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<QueuedTask>()
            .replicate::<ActiveTask>()
            .add_mapped_client_reflect_event::<TaskRequest, TaskRequestSerializer, TaskRequestDeserializer>()
            .add_client_event::<TaskCancel>()
            .add_systems(
                (
                    Self::list_system.run_if(action_just_pressed(Action::Confirm)),
                    Self::cleanup_system,

                )
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::queue_system,
                    Self::activation_system,
                    Self::cancelation_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl TaskPlugin {
    fn list_system(
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
        task_components: Res<TaskComponents>,
        registry: Res<AppTypeRegistry>,
        mut task_events: ResMut<Events<FromClient<TaskRequest>>>,
        actors: Query<(), With<Actor>>,
    ) {
        let registry = registry.read();
        for event in task_events.drain().map(|event| event.event) {
            if actors.get(event.entity).is_err() {
                error!("entity {:?} is not an actor", event.entity);
                continue;
            }

            // TODO 0.11: use `Reflect::get_represented_type_info`.
            let Some(registration) = registry.get_with_name(event.task.type_name()) else {
                error!("{} should be registered", event.task.type_name());
                continue;
            };

            if !task_components.contains(&registration.type_id()) {
                error!("{:?} is not a task", event.task.type_name());
                continue;
            }

            commands.entity(event.entity).with_children(|parent| {
                parent
                    .spawn((Replication, QueuedTask))
                    .insert_reflect([event.task.into_reflect()]);
            });
        }
    }

    fn activation_system(
        mut commands: Commands,
        active_tasks: Query<&TaskGroups, With<ActiveTask>>,
        queued_tasks: Query<(Entity, &TaskGroups), With<QueuedTask>>,
        actors: Query<&Children, With<Actor>>,
    ) {
        for children in &actors {
            let current_groups = active_tasks
                .iter_many(children)
                .copied()
                .reduce(|acc, groups| acc & groups)
                .unwrap_or_default();

            if let Some((task_entity, _)) = queued_tasks
                .iter_many(children)
                .find(|(_, groups)| !groups.intersects(current_groups))
            {
                commands
                    .entity(task_entity)
                    .remove::<QueuedTask>()
                    .insert(ActiveTask);
            }
        }
    }

    fn cancelation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<TaskCancel>>,
        queued_tasks: Query<(), With<QueuedTask>>,
        active_tasks: Query<(), With<ActiveTask>>,
    ) {
        for event in cancel_events.iter().map(|event| &event.event) {
            if queued_tasks.get(event.0).is_ok() {
                commands.entity(event.0).despawn();
            } else if active_tasks.get(event.0).is_ok() {
                commands.entity(event.0).insert(CancelledTask);
            } else {
                error!("entity {:?} is not a task", event.0);
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut removed_lists: RemovedComponents<TaskList>,
        children: Query<&Children>,
        tasks: Query<Entity, With<ListedTask>>,
    ) {
        for list_entity in &mut removed_lists {
            if let Ok(children) = children.get(list_entity) {
                for task_entity in tasks.iter_many(children) {
                    commands.entity(task_entity).despawn();
                }
            }
        }
    }
}

/// List of tasks assigned to entity.
#[derive(Component, Default, Deref, DerefMut)]
struct Tasks(Vec<Entity>);

/// Marker that indicates that the entity contains list of possible tasks as children.
///
/// Added when clicking on objects.
#[derive(Component)]
pub(crate) struct TaskList;

/// Marker for a task that is a children of [`TaskList`].
#[derive(Component)]
pub(crate) struct ListedTask;

/// Marker for a task that was queued.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct QueuedTask;

/// Marker for a task that is currently active.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct ActiveTask;

/// Marker for a task that was cancelled.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(super) struct CancelledTask;

bitflags! {
    #[derive(Default, Component)]
    pub(crate) struct TaskGroups: u8 {
        const LEFT_HAND = 0b00000001;
        const RIGHT_HAND = 0b00000010;
        const BOTH_HANDS = Self::LEFT_HAND.bits() | Self::RIGHT_HAND.bits();
        const LEGS = 0b00000100;
    }
}

pub(super) trait AppTaskExt {
    fn register_task<T: Component + GetTypeRegistration>(&mut self) -> &mut Self;
}

impl AppTaskExt for App {
    fn register_task<T: Component + GetTypeRegistration>(&mut self) -> &mut Self {
        self.world
            .get_resource_or_insert_with(TaskComponents::default)
            .push(TypeId::of::<T>());
        self.replicate::<T>()
    }
}

#[derive(Resource, Deref, DerefMut, Default)]
pub(crate) struct TaskComponents(pub(crate) Vec<TypeId>);

/// An event of canceling the specified task.
///
/// Emitted by players.
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct TaskCancel(pub(crate) Entity);

impl MapEntities for TaskCancel {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct TaskRequest {
    pub(crate) entity: Entity,
    pub(crate) task: Box<dyn Reflect>,
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
        event: &'a TaskRequest,
        registry: &'a TypeRegistryInternal,
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
        let task = seq
            .next_element_seed(UntypedReflectDeserializer::new(self.registry))?
            .ok_or_else(|| de::Error::invalid_length(TaskRequestField::Task as usize, &self))?;

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
}
