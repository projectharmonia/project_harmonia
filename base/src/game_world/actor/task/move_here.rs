use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{ActiveTask, AvailableTasks, Task, TaskAppExt, TaskGroups};
use crate::{
    core::GameState,
    game_world::{
        actor::Movement,
        city::Ground,
        navigation::{NavDestination, Navigation},
    },
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.add_task::<MoveHere>()
            .add_observer(Self::add_to_list)
            .add_observer(Self::activate)
            .add_systems(Update, Self::finish.run_if(in_state(GameState::InGame)));
    }
}

impl MoveHerePlugin {
    fn add_to_list(
        trigger: Trigger<OnAdd, AvailableTasks>,
        mut commands: Commands,
        available_tasks: Single<&AvailableTasks>,
        grounds: Query<(), With<Ground>>,
    ) {
        if grounds.get(available_tasks.interaction_entity).is_err() {
            return;
        }

        debug!("listing tasks");
        commands.entity(trigger.entity()).with_children(|parent| {
            parent.spawn((
                Name::new("Walk here"),
                MoveHere {
                    endpoint: available_tasks.click_point,
                    movement: Movement::Walk,
                },
            ));
            parent.spawn((
                Name::new("Run here"),
                MoveHere {
                    endpoint: available_tasks.click_point,
                    movement: Movement::Run,
                },
            ));
        });
    }

    fn activate(
        trigger: Trigger<OnAdd, ActiveTask>,
        mut actors: Query<(&mut Navigation, &mut NavDestination)>,
        tasks: Query<(&Parent, &MoveHere)>,
    ) {
        let Ok((parent, move_here)) = tasks.get(trigger.entity()) else {
            return;
        };

        debug!("starting movement");
        let (mut navigation, mut dest) = actors
            .get_mut(**parent)
            .expect("actors should have navigation component");
        *navigation = Navigation::new(move_here.movement.speed());
        **dest = Some(move_here.endpoint);
    }

    fn finish(
        mut commands: Commands,
        actors: Query<&NavDestination>,
        tasks: Query<(Entity, &Parent), (With<MoveHere>, With<ActiveTask>)>,
    ) {
        for (task_entity, parent) in &tasks {
            let dest = actors
                .get(**parent)
                .expect("actors should have always have destination");
            if dest.is_none() {
                debug!("ending movement");
                commands.entity(task_entity).despawn();
            }
        }
    }
}

#[derive(Clone, Reflect, Component, Copy, Deserialize, Serialize)]
#[reflect(Component)]
#[require(Task, TaskGroups(|| TaskGroups::LEGS))]
struct MoveHere {
    endpoint: Vec3,
    movement: Movement,
}
