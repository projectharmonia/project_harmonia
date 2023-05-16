use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;

use crate::core::{
    actor::{
        movement::{Movement, MovementBundle},
        Actor, ActorAnimation,
    },
    animation::AnimationEnded,
    asset_handles::AssetHandles,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    navigation::{following::Following, Navigation},
    task::{ReflectTask, Task, TaskList},
};

pub(super) struct TellSecretPlugin;

impl Plugin for TellSecretPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<TellSecret>()
            .replicate::<ListenSecret>()
            .register_component_as::<dyn Task, TellSecret>()
            .register_component_as::<dyn Task, ListenSecret>()
            .add_systems(
                (
                    Self::tasks_system,
                    Self::activation_system,
                    Self::tell_system,
                    Self::finish_system,
                    Self::cancellation_system,
                    Self::cleanup_system,
                )
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            );
    }
}

impl TellSecretPlugin {
    fn tasks_system(
        mut actors: Query<
            (Entity, &mut TaskList),
            (With<Actor>, With<CursorHover>, Added<TaskList>),
        >,
    ) {
        for (entity, mut task_list) in &mut actors {
            task_list.push(Box::new(TellSecret(entity)));
        }
    }

    fn activation_system(
        mut commands: Commands,
        actors: Query<(Entity, &TellSecret), Added<TellSecret>>,
    ) {
        for (entity, tell_secret) in &actors {
            commands.entity(entity).insert((
                MovementBundle::new(Movement::Walk).with_offset(0.5),
                Following(tell_secret.0),
            ));
        }
    }

    fn tell_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut tell_actors: Query<(Entity, &TellSecret, &Transform, &mut Handle<AnimationClip>)>,
        mut listen_actors: Query<(&mut Transform, &mut Handle<AnimationClip>), Without<TellSecret>>,
    ) {
        for entity in &mut removed_navigations {
            if let Ok((entity, tell_secret, tell_transform, mut tell_handle)) =
                tell_actors.get_mut(entity)
            {
                commands.entity(entity).insert(TellingSecret);
                *tell_handle = actor_animations.handle(ActorAnimation::TellSecret);

                commands.entity(tell_secret.0).insert(ListenSecret(entity));
                let (mut listen_transform, mut listen_handle) = listen_actors
                    .get_mut(tell_secret.0)
                    .expect("listener should have animation");
                listen_transform.look_at(tell_transform.translation, Vec3::Y);
                *listen_handle = actor_animations.handle(ActorAnimation::ThoughtfulNod);
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut end_events: EventReader<AnimationEnded>,
        mut actors: Query<Entity, With<TellingSecret>>,
    ) {
        for event in &mut end_events {
            if let Ok(entity) = actors.get_mut(event.0) {
                commands.entity(entity).remove::<TellSecret>();
            }
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        mut removed_listens: RemovedComponents<ListenSecret>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut animation_handles: Query<&mut Handle<AnimationClip>>,
        actors: Query<(Entity, &TellSecret)>,
    ) {
        for listen_entity in &mut removed_listens {
            if let Some((tell_entity, _)) = actors
                .iter()
                .find(|(_, tell_secret)| tell_secret.0 == listen_entity)
            {
                commands.entity(tell_entity).remove::<TellSecret>();
            }

            if let Ok(mut animation_handle) = animation_handles.get_mut(listen_entity) {
                *animation_handle = actor_animations.handle(ActorAnimation::Idle);
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut removed_tells: RemovedComponents<TellSecret>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut animation_handles: Query<&mut Handle<AnimationClip>>,
        actors: Query<(Entity, &ListenSecret)>,
    ) {
        for tell_entity in &mut removed_tells {
            if let Some((listen_entity, _)) = actors
                .iter()
                .find(|(_, listen_secret)| listen_secret.0 == tell_entity)
            {
                commands.entity(listen_entity).remove::<ListenSecret>();
            }

            if let Ok(mut animation_handle) = animation_handles.get_mut(tell_entity) {
                *animation_handle = actor_animations.handle(ActorAnimation::Idle);

                commands
                    .entity(tell_entity)
                    .remove::<(Navigation, TellingSecret)>();
            }
        }
    }
}

/// Indicates that the actor is currently telling a secret.
#[derive(Component)]
struct TellingSecret;

#[derive(Debug, Reflect, Component)]
#[reflect(Component, Task)]
struct TellSecret(Entity);

impl Task for TellSecret {
    fn name(&self) -> &str {
        "Tell secret"
    }
}

impl FromWorld for TellSecret {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

#[derive(Debug, Reflect, Component)]
#[reflect(Component, Task)]
struct ListenSecret(Entity);

impl Task for ListenSecret {
    fn name(&self) -> &str {
        "Tell secret"
    }
}

impl FromWorld for ListenSecret {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}
