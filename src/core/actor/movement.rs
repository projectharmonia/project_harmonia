use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

use super::{human_animation::HumanAnimation, Sex};
use crate::core::{
    actor::Players,
    family::FamilyMode,
    game_state::GameState,
    ground::Ground,
    task::{Task, TaskActivation, TaskCancel, TaskList, TaskRequest, TaskRequestKind},
};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_component_as::<dyn Task, Walk>()
            .add_system(
                Self::tasks_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::activation_system,
                    Self::cancellation_system,
                    Self::movement_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl MovementPlugin {
    fn tasks_system(mut ground: Query<&mut TaskList, (With<Ground>, Added<TaskList>)>) {
        if let Ok(mut task_list) = ground.get_single_mut() {
            task_list.tasks.push(TaskRequestKind::Walk);
        }
    }

    fn activation_system(
        mut commands: Commands,
        mut activation_events: EventReader<TaskActivation>,
        mut actors: Query<(&mut HumanAnimation, &Sex)>,
    ) {
        for TaskActivation { entity, task } in activation_events.iter().copied() {
            if let TaskRequest::Walk(position) = task {
                let (mut animation, sex) = actors
                    .get_mut(entity)
                    .expect("actors should always have assigned animation");
                let walk_animation = match sex {
                    Sex::Male => HumanAnimation::MaleWalk,
                    Sex::Female => HumanAnimation::FemaleWalk,
                };
                *animation = walk_animation;
                commands.entity(entity).insert(Walk(position));
            }
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<FromClient<TaskCancel>>,
        actors: Query<(Entity, &Players)>,
    ) {
        for FromClient { client_id, event } in cancel_events.iter().copied() {
            if let Some((entity, _)) = actors
                .iter()
                .find(|(_, players)| players.contains(&client_id))
            {
                if let TaskRequestKind::Walk = event.0 {
                    commands.entity(entity).remove::<Walk>();
                }
            } else {
                error!("no controlled entity for {event:?} for client {client_id}");
            }
        }
    }

    fn movement_system(
        mut commands: Commands,
        time: Res<Time>,
        mut actors: Query<(Entity, &mut Transform, &mut HumanAnimation, &Walk)>,
    ) {
        for (entity, mut transform, mut animation, walk) in &mut actors {
            let direction = walk.0 - transform.translation;

            if direction.length() < 0.1 {
                commands.entity(entity).remove::<Walk>();
                *animation = HumanAnimation::Idle;
            } else {
                const ROTATION_SPEED: f32 = 10.0;
                const WALK_SPEED: f32 = 2.0;
                let delta_secs = time.delta_seconds();
                let target_rotation = transform.looking_to(direction, Vec3::Y).rotation;

                transform.translation += direction.normalize() * WALK_SPEED * delta_secs;
                transform.rotation = transform
                    .rotation
                    .slerp(target_rotation, ROTATION_SPEED * delta_secs);
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
pub(crate) struct Walk(pub(crate) Vec3);

impl Task for Walk {
    fn kind(&self) -> TaskRequestKind {
        TaskRequestKind::Walk
    }
}
