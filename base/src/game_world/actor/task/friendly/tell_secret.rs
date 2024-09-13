use bevy::{
    animation::RepeatAnimation,
    ecs::{entity::MapEntities, reflect::ReflectMapEntities},
    prelude::*,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    asset::collection::Collection,
    core::GameState,
    game_world::{
        actor::{
            animation_state::{AnimationState, Montage, MontageFinished},
            task::{linked_task::LinkedTask, Task, TaskGroups, TaskList, TaskListSet, TaskState},
            Actor, ActorAnimation, Movement,
        },
        hover::Hovered,
        navigation::{following::Following, NavDestination, NavSettings},
    },
};

pub(super) struct TellSecretPlugin;

impl Plugin for TellSecretPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<TellSecret>()
            .register_type::<ListenSecret>()
            .replicate::<TellSecret>()
            .replicate::<ListenSecret>()
            .add_systems(
                Update,
                (
                    Self::add_to_list.in_set(TaskListSet),
                    Self::start_following.run_if(server_or_singleplayer),
                    Self::start_telling,
                    Self::start_listening,
                    Self::finish,
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl TellSecretPlugin {
    fn add_to_list(
        mut list_events: EventWriter<TaskList>,
        mut actors: Query<Entity, (With<Actor>, With<Hovered>)>,
    ) {
        if let Ok(entity) = actors.get_single_mut() {
            list_events.send(TellSecret(entity).into());
        }
    }

    fn start_following(
        mut commands: Commands,
        mut actors: Query<&mut NavSettings>,
        tasks: Query<(&TellSecret, &Parent, &TaskState), Changed<TaskState>>,
    ) {
        for (tell_secret, parent, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let mut nav_settings = actors
                    .get_mut(**parent)
                    .expect("actors should have navigation component");
                *nav_settings = NavSettings::new(Movement::Walk.speed()).with_offset(0.5);

                commands.entity(**parent).insert(Following(tell_secret.0));
            }
        }
    }

    fn start_telling(
        mut commands: Commands,
        actor_animations: Res<Collection<ActorAnimation>>,
        mut actors: Query<
            (Entity, &Children, &NavDestination, &mut AnimationState),
            Changed<NavDestination>,
        >,
        tasks: Query<(Entity, &TellSecret, &TaskState)>,
    ) {
        for (actor_entity, children, dest, mut animator) in &mut actors {
            if !dest.is_none() {
                continue;
            }

            let Some((tell_entity, tell_secret, _)) = tasks
                .iter_many(children)
                .find(|(.., &task_state)| task_state == TaskState::Active)
            else {
                continue;
            };

            let montage = Montage::new(actor_animations.handle(ActorAnimation::TellSecret));
            animator.play_montage(montage);

            // TODO: Handle cancellation of currently active tasks.
            commands.entity(tell_secret.0).with_children(|parent| {
                parent.spawn(ListenSecretBundle::new(actor_entity, tell_entity));
            });
        }
    }

    fn start_listening(
        actor_animations: Res<Collection<ActorAnimation>>,
        tasks: Query<(&ListenSecret, &Parent, &TaskState), Changed<TaskState>>,
        mut actors: Query<(&mut Transform, &mut AnimationState)>,
    ) {
        for (listen_secret, parent, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let (&tell_transform, _) = actors
                    .get(listen_secret.0)
                    .expect("teller should have transform");
                let (mut listen_transform, mut animation_state) = actors
                    .get_mut(**parent)
                    .expect("listener should have transform and animation");

                listen_transform.look_at(tell_transform.translation, Vec3::Y);
                let montage = Montage::new(actor_animations.handle(ActorAnimation::ThoughtfulNod))
                    .with_repeat(RepeatAnimation::Forever);
                animation_state.play_montage(montage);
            }
        }
    }

    fn finish(
        mut commands: Commands,
        mut finish_events: EventReader<MontageFinished>,
        children: Query<&Children>,
        tasks: Query<(Entity, &TaskState), With<TellSecret>>,
    ) {
        for children in children.iter_many(finish_events.read().map(|event| event.0)) {
            if let Some((entity, _)) = tasks
                .iter_many(children)
                .find(|(_, &task_state)| task_state == TaskState::Active)
            {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component, MapEntities)]
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

impl MapEntities for TellSecret {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component, MapEntities)]
struct ListenSecret(Entity);

impl FromWorld for ListenSecret {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

impl MapEntities for ListenSecret {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

#[derive(Bundle)]
struct ListenSecretBundle {
    task_groups: TaskGroups,
    task_state: TaskState,
    listen_secret: ListenSecret,
    link: LinkedTask,
}

impl ListenSecretBundle {
    fn new(actor_entity: Entity, task_entity: Entity) -> Self {
        Self {
            task_groups: TaskGroups::LEGS,
            task_state: Default::default(),
            listen_secret: ListenSecret(actor_entity),
            link: LinkedTask(task_entity),
        }
    }
}
