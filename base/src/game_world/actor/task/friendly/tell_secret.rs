use bevy::{animation::RepeatAnimation, ecs::entity::MapEntities, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    asset::collection::Collection,
    game_world::{
        actor::{
            animation_state::{AnimationState, Montage, MontageFinished},
            task::{
                linked_task::LinkedTask, ActiveTask, AvailableTasks, Task, TaskAppExt, TaskGroups,
            },
            Actor, ActorAnimation, Movement,
        },
        navigation::{following::Following, Navigation},
    },
};

pub(super) struct TellSecretPlugin;

impl Plugin for TellSecretPlugin {
    fn build(&self, app: &mut App) {
        app.add_mapped_task::<TellSecret>()
            .add_mapped_task::<ListenSecret>()
            .add_observer(add_to_list)
            .add_observer(activate)
            .add_observer(start_telling)
            .add_observer(start_listening)
            .add_observer(finish);
    }
}

fn add_to_list(
    trigger: Trigger<OnAdd, AvailableTasks>,
    mut commands: Commands,
    available_tasks: Single<&AvailableTasks>,
    actors: Query<(), With<Actor>>,
) {
    if actors.get(available_tasks.interaction_entity).is_ok() {
        debug!("listing task");
        commands.entity(trigger.entity()).with_children(|parent| {
            parent.spawn(TellSecret {
                target_entity: available_tasks.interaction_entity,
            });
        });
    }
}

fn activate(
    trigger: Trigger<OnAdd, ActiveTask>,
    mut commands: Commands,
    mut actors: Query<&mut Navigation>,
    tasks: Query<(&Parent, &TellSecret)>,
) {
    let Ok((parent, tell_secret)) = tasks.get(trigger.entity()) else {
        return;
    };

    let mut navigation = actors
        .get_mut(**parent)
        .expect("actors should have navigation component");
    *navigation = Navigation::new(Movement::Walk.speed()).with_offset(0.5);

    commands
        .entity(**parent)
        .insert(Following(tell_secret.target_entity));
}

fn start_telling(
    trigger: Trigger<OnRemove, Following>,
    mut commands: Commands,
    actor_animations: Res<Collection<ActorAnimation>>,
    mut actors: Query<(&Children, &mut AnimationState)>,
    mut tasks: Query<(Entity, &TellSecret, &mut LinkedTask), With<ActiveTask>>,
) {
    let (children, mut animation_state) = actors.get_mut(trigger.entity()).unwrap();

    if let Some((tell_entity, tell_secret, mut linked_task)) =
        tasks.iter_many_mut(children).fetch_next()
    {
        let montage = Montage::new(actor_animations.handle(ActorAnimation::TellSecret));
        animation_state.play_montage(montage);

        // TODO: Handle cancellation of currently active tasks.
        commands
            .entity(tell_secret.target_entity)
            .with_children(|parent| {
                let listen_entity = parent
                    .spawn((
                        LinkedTask(Some(tell_entity)),
                        ListenSecret {
                            teller_entity: trigger.entity(),
                        },
                    ))
                    .id();

                **linked_task = Some(listen_entity)
            });
    }
}

fn start_listening(
    trigger: Trigger<OnAdd, ActiveTask>,
    actor_animations: Res<Collection<ActorAnimation>>,
    tasks: Query<(&Parent, &ListenSecret)>,
    mut actors: Query<(&mut Transform, &mut AnimationState)>,
) {
    let Ok((parent, listen_secret)) = tasks.get(trigger.entity()) else {
        return;
    };

    let (&teller_transform, _) = actors
        .get(listen_secret.teller_entity)
        .expect("teller should have transform");

    let (mut listener_transform, mut animation_state) = actors
        .get_mut(**parent)
        .expect("listener should have transform and animation");

    listener_transform.look_at(teller_transform.translation, Vec3::Y);
    let montage = Montage::new(actor_animations.handle(ActorAnimation::ThoughtfulNod))
        .with_repeat(RepeatAnimation::Forever);
    animation_state.play_montage(montage);
}

fn finish(
    trigger: Trigger<MontageFinished>,
    mut commands: Commands,
    children: Query<&Children>,
    tasks: Query<Entity, (With<TellSecret>, With<ActiveTask>)>,
) {
    let Ok(children) = children.get(trigger.entity()) else {
        return;
    };

    if let Some(task_entity) = tasks.iter_many(children).next() {
        commands.entity(task_entity).despawn();
    }
}

#[derive(Component, Reflect, Deserialize, Serialize, Clone, Copy)]
#[reflect(Component)]
#[require(
    Name(|| Name::new("Tell secret")),
    Task,
    LinkedTask,
    TaskGroups(|| TaskGroups::LEGS),
)]
struct TellSecret {
    target_entity: Entity,
}

impl MapEntities for TellSecret {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.target_entity = entity_mapper.map_entity(self.target_entity);
    }
}

#[derive(Component, Reflect, Deserialize, Serialize, Clone, Copy)]
#[reflect(Component)]
#[require(
    Name(|| Name::new("Listen secret")),
    Task,
    TaskGroups(|| TaskGroups::LEGS),
)]
struct ListenSecret {
    teller_entity: Entity,
}

impl MapEntities for ListenSecret {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.teller_entity = entity_mapper.map_entity(self.teller_entity);
    }
}
