mod friendly;
mod linked_task;
mod move_here;

use std::any;

use bevy::{ecs::entity::MapEntities, prelude::*, reflect::GetTypeRegistration};
use bevy_replicon::prelude::*;
use bitflags::bitflags;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::{animation_state::AnimationState, Actor, ActorTaskGroups, SelectedActor};
use crate::game_world::{city::ActiveCity, family::FamilyMode, navigation::NavDestination};
use friendly::FriendlyPlugins;
use linked_task::LinkedTaskPlugin;
use move_here::MoveHerePlugin;

pub(super) struct TaskPlugin;

impl Plugin for TaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FriendlyPlugins, LinkedTaskPlugin, MoveHerePlugin))
            .replicate::<ActiveTask>()
            .add_client_event::<TaskCancel>(ChannelKind::Unordered)
            .add_observer(Self::spawn_available.never_param_warn())
            .add_observer(Self::cleanup)
            .add_systems(
                PreUpdate,
                Self::cancel
                    .after(ClientSet::Receive)
                    .run_if(server_or_singleplayer),
            )
            .add_systems(
                PostUpdate,
                Self::activate_queued.run_if(server_or_singleplayer),
            );
    }
}

impl TaskPlugin {
    fn spawn_available(
        mut trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        family_mode: Res<State<FamilyMode>>,
        city_transform: Single<&GlobalTransform, With<ActiveCity>>,
        tasks_entity: Option<Single<Entity, With<AvailableTasks>>>,
    ) {
        if trigger.event().button != PointerButton::Primary {
            return;
        }
        if *family_mode != FamilyMode::Life {
            return;
        }
        let Some(mut click_point) = trigger.event().hit.position else {
            // Consider only world clicking.
            return;
        };
        trigger.propagate(false);
        debug!("generating available tasks");

        click_point = city_transform
            .affine()
            .inverse()
            .transform_point3(click_point);

        // Remove previous.
        if let Some(tasks_entity) = tasks_entity {
            commands.entity(*tasks_entity).despawn();
        }

        commands.entity(trigger.entity()).with_children(|parent| {
            parent.spawn(AvailableTasks {
                interaction_entity: trigger.entity(),
                click_point,
            });
        });
    }

    fn activate_queued(
        mut commands: Commands,
        tasks: Query<(Entity, &Name, &TaskGroups), Without<ActiveTask>>,
        mut actors: Query<(&Children, &mut ActorTaskGroups)>,
    ) {
        for (children, mut actor_groups) in &mut actors {
            for (entity, name, &groups) in tasks.iter_many(children) {
                if !groups.intersects(**actor_groups) {
                    debug!("activating '{name}' for `{entity}`");
                    actor_groups.insert(groups);
                    commands.entity(entity).insert(ActiveTask);
                }
            }
        }
    }

    fn cancel(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<TaskCancel>>,
        tasks: Query<(), With<Task>>,
    ) {
        for FromClient { client_id, event } in cancel_events.read() {
            if tasks.get(**event).is_ok() {
                info!("`{client_id:?}` cancels task `{}`", **event);
                commands.entity(**event).despawn();
            } else {
                error!("task {:?} is not active", **event);
            }
        }
    }

    fn cleanup(
        trigger: Trigger<OnRemove, TaskGroups>,
        tasks: Query<(&Parent, &TaskGroups), With<ActiveTask>>,
        mut actors: Query<(
            &mut ActorTaskGroups,
            &mut NavDestination,
            &mut AnimationState,
        )>,
    ) {
        let (parent, &task_groups) = tasks.get(trigger.entity()).unwrap();
        let Ok((mut actor_groups, mut dest, mut animation_state)) = actors.get_mut(**parent) else {
            return;
        };

        debug!("removing `{:?}` from actor `{}`", task_groups, **parent);
        actor_groups.remove(task_groups);

        if task_groups.contains(TaskGroups::LEGS) {
            debug!("cancelling task navigation");
            **dest = None;
        }

        animation_state.stop_montage();
    }
}

#[derive(Component)]
/// Stores available tasks for an entity, triggered by picking.
pub struct AvailableTasks {
    // TODO 0.16: Use `Parent` when hierarchy will be accessible in observers.
    interaction_entity: Entity,
    click_point: Vec3,
}

#[derive(Component, Default)]
#[require(Name, TaskGroups, ParentSync, Replicated)]
pub struct Task;

#[derive(Component, Serialize, Deserialize)]
pub struct ActiveTask;

bitflags! {
    #[derive(Default, Component, Clone, Copy, Debug)]
    pub(super) struct TaskGroups: u8 {
        const LEFT_HAND = 0b00000001;
        const RIGHT_HAND = 0b00000010;
        const BOTH_HANDS = Self::LEFT_HAND.bits() | Self::RIGHT_HAND.bits();
        const LEGS = 0b00000100;
    }
}

/// An event for selecting a task from menu.
#[derive(Deserialize, Event, Serialize)]
pub struct TaskSelect;

/// An event of canceling the specified task.
///
/// Emitted by players.
#[derive(Deserialize, Event, Serialize, Deref)]
pub struct TaskCancel(pub Entity);

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub struct TaskRequest<C> {
    pub entity: Entity,
    pub task: C,
}

impl<C> MapEntities for TaskRequest<C> {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.entity = entity_mapper.map_entity(self.entity);
    }
}

#[derive(Event, Clone, Copy, Serialize, Deserialize)]
pub struct MappedTaskRequest<C> {
    pub entity: Entity,
    pub task: C,
}

impl<C: MapEntities> MapEntities for MappedTaskRequest<C> {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.entity = entity_mapper.map_entity(self.entity);
        self.task.map_entities(entity_mapper);
    }
}

trait Request<C> {
    fn new(entity: Entity, task: C) -> Self;
    fn entity(&self) -> Entity;
    fn take_task(self) -> C;
}

impl<C> Request<C> for TaskRequest<C> {
    fn new(entity: Entity, task: C) -> Self {
        Self { entity, task }
    }

    fn entity(&self) -> Entity {
        self.entity
    }

    fn take_task(self) -> C {
        self.task
    }
}

impl<C> Request<C> for MappedTaskRequest<C> {
    fn new(entity: Entity, task: C) -> Self {
        Self { entity, task }
    }

    fn entity(&self) -> Entity {
        self.entity
    }

    fn take_task(self) -> C {
        self.task
    }
}

pub(super) trait TaskAppExt {
    fn add_task<C>(&mut self) -> &mut Self
    where
        C: Component + GetTypeRegistration + Copy + Serialize + DeserializeOwned;

    fn add_mapped_task<C>(&mut self) -> &mut Self
    where
        C: Component + GetTypeRegistration + Copy + Serialize + DeserializeOwned + MapEntities;
}

impl TaskAppExt for App {
    fn add_task<C>(&mut self) -> &mut Self
    where
        C: Component + GetTypeRegistration + Copy + Serialize + DeserializeOwned,
    {
        self.register_type::<C>()
            .replicate::<C>()
            .add_client_event::<TaskRequest<C>>(ChannelKind::Ordered)
            .add_observer(request::<TaskRequest<C>, _>)
            .add_systems(
                PreUpdate,
                queue::<TaskRequest<C>, _>
                    .after(ClientSet::Receive)
                    .run_if(server_or_singleplayer),
            )
    }

    fn add_mapped_task<C>(&mut self) -> &mut Self
    where
        C: Component + GetTypeRegistration + Copy + Serialize + DeserializeOwned + MapEntities,
    {
        self.register_type::<C>()
            .replicate_mapped::<C>()
            .add_mapped_client_event::<MappedTaskRequest<C>>(ChannelKind::Ordered)
            .add_observer(request::<MappedTaskRequest<C>, _>)
            .add_systems(
                PreUpdate,
                queue::<MappedTaskRequest<C>, _>
                    .after(ClientSet::Receive)
                    .run_if(server_or_singleplayer),
            )
    }
}

fn request<R, C>(
    trigger: Trigger<TaskSelect>,
    mut commands: Commands,
    mut request_events: EventWriter<R>,
    tasks: Query<(&Name, &C)>,
    tasks_entity: Single<Entity, With<AvailableTasks>>,
    selected_entity: Single<Entity, With<SelectedActor>>,
) where
    R: Request<C> + Event,
    C: Component + Copy,
{
    let Ok((name, &task)) = tasks.get(trigger.entity()) else {
        return;
    };

    info!("selecting `{name}`");
    request_events.send(R::new(*selected_entity, task));
    commands.entity(*tasks_entity).despawn_recursive();
}

fn queue<R, C>(
    mut commands: Commands,
    mut request_events: EventReader<FromClient<R>>,
    actors: Query<(), With<Actor>>,
) where
    R: Request<C> + Copy + Event,
    C: Component + Copy,
{
    for FromClient { client_id, event } in request_events.read() {
        if actors.get(event.entity()).is_ok() {
            info!("`{client_id:?}` requests task `{}`", any::type_name::<C>());
            commands.entity(event.entity()).with_children(|parent| {
                parent.spawn(event.take_task());
            });
        } else {
            error!("entity {:?} is not an actor", event.entity());
        }
    }
}
