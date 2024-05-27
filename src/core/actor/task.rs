mod buy_lot;
mod friendly;
mod linked_task;
mod move_here;

use std::{fmt::Debug, io::Cursor};

use bevy::{
    ecs::{entity::MapEntities, reflect::ReflectCommandExt},
    prelude::*,
    reflect::serde::{ReflectSerializer, UntypedReflectDeserializer},
};
use bevy_replicon::{
    core::ctx::{ClientSendCtx, ServerReceiveCtx},
    prelude::*,
};
use bincode::{DefaultOptions, ErrorKind, Options};
use bitflags::bitflags;
use leafwing_input_manager::common_conditions::action_just_pressed;
use serde::{de::DeserializeSeed, Deserialize, Serialize};

use crate::core::{
    action::Action,
    actor::Actor,
    animation_state::AnimationState,
    family::FamilyMode,
    game_state::GameState,
    navigation::{ComputePath, NavPath},
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
        .add_client_event::<TaskCancel>(ChannelKind::Unordered)
        .add_client_event_with(
            ChannelKind::Unordered,
            serialize_task_request,
            deserialize_task_request,
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
            (Self::request, Self::cancel)
                .after(ClientSet::Receive)
                .run_if(has_authority),
        )
        .add_systems(
            PostUpdate,
            (Self::despawn_cancelled, Self::activate_queued).run_if(has_authority),
        );
    }
}

impl TaskPlugin {
    fn request(
        mut commands: Commands,
        mut request_events: ResMut<Events<FromClient<TaskRequest>>>,
        actors: Query<(), With<Actor>>,
    ) {
        for event in request_events.drain().map(|event| event.event) {
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

    fn activate_queued(
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

    fn cancel(
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

    fn despawn_cancelled(
        mut commands: Commands,
        tasks: Query<(Entity, &Parent, &TaskGroups, &TaskState), Changed<TaskState>>,
        mut actors: Query<(&mut NavPath, &mut AnimationState)>,
    ) {
        for (entity, parent, groups, &task_state) in &tasks {
            if task_state == TaskState::Cancelled {
                let (mut nav_path, mut animation_state) = actors
                    .get_mut(**parent)
                    .expect("actor should have animaition state");

                if groups.contains(TaskGroups::LEGS) {
                    nav_path.clear();
                    commands.entity(**parent).remove::<ComputePath>();
                }

                animation_state.stop();

                commands.entity(entity).despawn();
            }
        }
    }
}

fn serialize_task_request(
    ctx: &mut ClientSendCtx,
    event: &TaskRequest,
    cursor: &mut Cursor<Vec<u8>>,
) -> bincode::Result<()> {
    let serializer = ReflectSerializer::new(event.task.as_reflect(), ctx.registry);
    DefaultOptions::new().serialize_into(&mut *cursor, &event.entity)?;
    DefaultOptions::new().serialize_into(cursor, &serializer)?;

    Ok(())
}

fn deserialize_task_request(
    ctx: &mut ServerReceiveCtx,
    cursor: &mut Cursor<&[u8]>,
) -> bincode::Result<TaskRequest> {
    let entity = DefaultOptions::new().deserialize_from(&mut *cursor)?;
    let mut deserializer = bincode::Deserializer::with_reader(cursor, DefaultOptions::new());
    let reflect = UntypedReflectDeserializer::new(ctx.registry).deserialize(&mut deserializer)?;
    let type_info = reflect.get_represented_type_info().unwrap();
    let type_path = type_info.type_path();
    let registration = ctx
        .registry
        .get(type_info.type_id())
        .ok_or_else(|| ErrorKind::Custom(format!("{type_path} is not registered")))?;
    let reflect_task = registration
        .data::<ReflectTask>()
        .ok_or_else(|| ErrorKind::Custom(format!("{type_path} doesn't have reflect(Task)")))?;
    let task = reflect_task
        .get_boxed(reflect)
        .map_err(|_| ErrorKind::Custom(format!("{type_path} is not a Task")))?;

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
    groups: TaskGroups,
    state: TaskState,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl TaskBundle {
    fn new(task: &dyn Task) -> Self {
        Self {
            groups: task.groups(),
            state: Default::default(),
            parent_sync: Default::default(),
            replication: Replicated,
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

impl MapEntities for TaskCancel {
    fn map_entities<T: EntityMapper>(&mut self, mapper: &mut T) {
        self.0 = mapper.map_entity(self.0);
    }
}

#[derive(Event)]
pub(crate) struct TaskRequest {
    pub(crate) entity: Entity,
    pub(crate) task: Box<dyn Task>,
}

impl MapEntities for TaskRequest {
    fn map_entities<T: EntityMapper>(&mut self, mapper: &mut T) {
        self.entity = mapper.map_entity(self.entity);
    }
}
