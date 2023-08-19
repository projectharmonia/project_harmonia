use bevy::prelude::*;
use bevy_replicon::prelude::*;

use crate::core::{
    actor::{
        movement::Movement,
        task::{linked_task::LinkedTask, Task, TaskGroups, TaskList, TaskListSet, TaskState},
        Actor, ActorAnimation,
    },
    animation_state::{AnimationFinished, AnimationState},
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
        for (tell_secret, parent, &task_state) in &tasks {
            if task_state == TaskState::Active {
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
        mut actors: Query<(&Children, &mut AnimationState)>,
        tasks: Query<(Entity, &TellSecret, &TaskState)>,
    ) {
        for actor_entity in &mut removed_navigations {
            let Ok((children, mut animation_state)) = actors.get_mut(actor_entity) else {
                continue;
            };

            let Some((tell_entity, tell_secret, _)) = tasks.iter_many(children).find(|(.., &task_state)| task_state == TaskState::Active) else {
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
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        tasks: Query<(&ListenSecret, &Parent, &TaskState), Changed<TaskState>>,
        mut actors: Query<(&mut Transform, &mut AnimationState)>,
    ) {
        for (listen_secret, parent, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let tell_transform: Transform = *actors
                    .get_component(listen_secret.0)
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
        for children in children.iter_many(finish_events.iter().map(|event| event.0)) {
            if let Some((entity, _)) = tasks
                .iter_many(children)
                .find(|(_, &task_state)| task_state == TaskState::Active)
            {
                commands.entity(entity).despawn();
            }
        }
    }
}

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
