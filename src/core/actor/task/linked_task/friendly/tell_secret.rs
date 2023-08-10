use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::core::{
    actor::{
        movement::Movement,
        task::{linked_task::LinkedTask, Task, TaskGroups, TaskList, TaskListSet, TaskState},
        Actor, ActorAnimation,
    },
    animation::AnimationEnded,
    asset_handles::AssetHandles,
    cursor_hover::CursorHover,
    game_world::WorldName,
    navigation::{following::Following, Navigation},
};

pub(super) struct TellSecretPlugin;

impl Plugin for TellSecretPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<TellSecret>()
            .replicate::<ListenSecret>()
            .add_systems(
                Update,
                (
                    Self::list_system.in_set(TaskListSet),
                    Self::tell_activation_system,
                    Self::tell_system,
                    Self::listen_activation_system,
                    Self::finish_system,
                    Self::tell_cancellation_system,
                    Self::cleanup_system,
                )
                    .run_if(resource_exists::<WorldName>()),
            );
    }
}

impl TellSecretPlugin {
    fn list_system(
        mut list_events: EventWriter<TaskList>,
        mut actors: Query<Entity, (With<Actor>, With<CursorHover>)>,
    ) {
        if let Ok(entity) = actors.get_single_mut() {
            list_events.send(TellSecret(entity).into());
        }
    }

    fn tell_activation_system(
        mut commands: Commands,
        tasks: Query<(&TellSecret, &Parent, &TaskState), Changed<TaskState>>,
    ) {
        for (tell_secret, parent, &state) in &tasks {
            if state == TaskState::Active {
                commands.entity(**parent).insert((
                    Navigation::new(Movement::Walk.speed()).with_offset(0.5),
                    Following(tell_secret.0),
                ));
            }
        }
    }

    fn tell_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<(&Children, &mut Handle<AnimationClip>)>,
        tasks: Query<(Entity, &TellSecret, &TaskState)>,
    ) {
        for actor_entity in &mut removed_navigations {
            let Ok((children, mut animation_handle)) = actors.get_mut(actor_entity) else {
                continue;
            };

            let Some((tell_entity, tell_secret, _)) = tasks.iter_many(children).find(|(.., &state)| state == TaskState::Active) else {
                continue;
            };

            commands.entity(actor_entity).insert(TellingSecret);
            *animation_handle = actor_animations.handle(ActorAnimation::TellSecret);

            // TODO: Handle cancellation of currently active tasks.
            commands.entity(tell_secret.0).with_children(|parent| {
                parent.spawn(ListenSecretBundle::new(actor_entity, tell_entity));
            });
        }
    }

    fn listen_activation_system(
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        tasks: Query<(&ListenSecret, &Parent), Added<ListenSecret>>,
        mut listen_actors: Query<
            (&mut Transform, &mut Handle<AnimationClip>),
            Without<TellingSecret>,
        >,
        tell_actors: Query<&Transform, With<TellingSecret>>,
    ) {
        for (tell_secret, parent) in &tasks {
            let (mut listen_transform, mut animation_handle) = listen_actors
                .get_mut(**parent)
                .expect("listener should have animation");
            let tell_transform = tell_actors
                .get(tell_secret.0)
                .expect("teller should have transform");

            listen_transform.look_at(tell_transform.translation, Vec3::Y);
            *animation_handle = actor_animations.handle(ActorAnimation::ThoughtfulNod);
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut end_events: EventReader<AnimationEnded>,
        tell_actors: Query<(), With<TellingSecret>>,
    ) {
        for event in &mut end_events {
            if tell_actors.get(event.0).is_ok() {
                commands.entity(event.0).insert(ToldSecret);
            }
        }
    }

    fn tell_cancellation_system(
        mut commands: Commands,
        tasks: Query<(&Parent, &TaskState), (Changed<TaskState>, With<TellSecret>)>,
    ) {
        for (parent, &state) in &tasks {
            if state == TaskState::Cancelled {
                commands.entity(**parent).insert(ToldSecret);
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut tell_actors: Query<(Entity, &Children, &mut Handle<AnimationClip>), Added<ToldSecret>>,
        mut listen_actors: Query<&mut Handle<AnimationClip>, Without<ToldSecret>>,
        listen_tasks: Query<(Entity, &Parent, &ListenSecret, &TaskState)>,
        tell_tasks: Query<(Entity, &TaskState), With<TellSecret>>,
    ) {
        for (teller_entity, children, mut animation_handle) in &mut tell_actors {
            if let Some((listen_entity, parent, ..)) =
                listen_tasks.iter().find(|(.., listen_secret, &state)| {
                    listen_secret.0 == teller_entity && state != TaskState::Queued
                })
            {
                commands.entity(listen_entity).despawn();
                let mut animation_handle = listen_actors
                    .get_mut(**parent)
                    .expect("actor should have animation handle");
                *animation_handle = actor_animations.handle(ActorAnimation::Idle);
            }

            *animation_handle = actor_animations.handle(ActorAnimation::Idle);

            commands
                .entity(teller_entity)
                .remove::<(Navigation, TellingSecret, ToldSecret)>();

            let (tell_entity, _) = tell_tasks
                .iter_many(children)
                .find(|(_, &state)| state != TaskState::Queued)
                .expect("actor should have tell secret task as a child");
            commands.entity(tell_entity).despawn();
        }
    }
}

/// Indicates that the actor is currently telling a secret.
#[derive(Component)]
struct TellingSecret;

/// Indicates that the actor just finished telling a secret.
#[derive(Component)]
struct ToldSecret;

#[derive(Debug, Reflect, Component)]
#[reflect(Component)]
struct TellSecret(Entity);

impl Task for TellSecret {
    fn name(&self) -> &str {
        "Tell secret"
    }

    fn groups(&self) -> TaskGroups {
        TaskGroups::LEGS
    }
}

impl FromWorld for TellSecret {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

#[derive(Debug, Reflect, Component)]
#[reflect(Component)]
struct ListenSecret(Entity);

impl FromWorld for ListenSecret {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

#[derive(Bundle)]
struct ListenSecretBundle {
    name: Name,
    task_groups: TaskGroups,
    task_state: TaskState,
    listen_secret: ListenSecret,
    link: LinkedTask,
}

impl ListenSecretBundle {
    fn new(actor_entity: Entity, task_entity: Entity) -> Self {
        Self {
            name: Name::new("Listen secret"),
            task_groups: TaskGroups::LEGS,
            task_state: Default::default(),
            listen_secret: ListenSecret(actor_entity),
            link: LinkedTask(task_entity),
        }
    }
}
