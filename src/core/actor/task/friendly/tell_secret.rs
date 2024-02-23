use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    actor::{
        movement_animation::Movement,
        task::{linked_task::LinkedTask, Task, TaskGroups, TaskList, TaskListSet, TaskState},
        Actor, ActorAnimation,
    },
    animation_state::{AnimationFinished, AnimationState},
    asset::collection::Collection,
    cursor_hover::CursorHover,
    game_world::WorldName,
    navigation::{following::Following, NavPath, Navigation},
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
                    Self::list_system.in_set(TaskListSet),
                    Self::tell_activation_system,
                    Self::tell_system,
                    Self::listen_activation_system,
                    Self::finish_system,
                )
                    .run_if(resource_exists::<WorldName>),
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
        mut actors: Query<&mut Navigation>,
        tasks: Query<(&TellSecret, &Parent, &TaskState), Changed<TaskState>>,
    ) {
        for (tell_secret, parent, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let mut navigation = actors
                    .get_mut(**parent)
                    .expect("actors should have navigation component");
                *navigation = Navigation::new(Movement::Walk.speed()).with_offset(0.5);

                commands.entity(**parent).insert(Following(tell_secret.0));
            }
        }
    }

    fn tell_system(
        mut commands: Commands,
        actor_animations: Res<Collection<ActorAnimation>>,
        mut actors: Query<(Entity, &Children, &NavPath, &mut AnimationState), Changed<NavPath>>,
        tasks: Query<(Entity, &TellSecret, &TaskState)>,
    ) {
        for (actor_entity, children, nav_path, mut animation_state) in &mut actors {
            if !nav_path.is_empty() {
                continue;
            }

            let Some((tell_entity, tell_secret, _)) = tasks
                .iter_many(children)
                .find(|(.., &task_state)| task_state == TaskState::Active)
            else {
                continue;
            };

            animation_state.play_once(actor_animations.handle(ActorAnimation::TellSecret));

            // TODO: Handle cancellation of currently active tasks.
            commands.entity(tell_secret.0).with_children(|parent| {
                parent.spawn(ListenSecretBundle::new(actor_entity, tell_entity));
            });
        }
    }

    fn listen_activation_system(
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
                animation_state.repeat(actor_animations.handle(ActorAnimation::ThoughtfulNod));
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut finish_events: EventReader<AnimationFinished>,
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

#[derive(Component, Deserialize, Reflect, Serialize)]
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
