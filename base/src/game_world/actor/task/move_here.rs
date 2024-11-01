use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{AvailableTasks, ListTasks, Task, TaskGroups, TaskState};
use crate::{
    core::GameState,
    game_world::{
        actor::Movement,
        city::Ground,
        navigation::{NavDestination, NavSettings},
    },
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveHere>()
            .replicate::<MoveHere>()
            .observe(Self::add_to_list)
            .add_systems(Update, Self::finish.run_if(in_state(GameState::InGame)))
            // Should run in `PostUpdate` to let tiles initialize.
            .add_systems(
                PostUpdate,
                Self::start_navigation.run_if(server_or_singleplayer),
            );
    }
}

impl MoveHerePlugin {
    fn add_to_list(
        trigger: Trigger<ListTasks>,
        mut available_tasks: ResMut<AvailableTasks>,
        grounds: Query<(), With<Ground>>,
    ) {
        if grounds.get(trigger.entity()).is_ok() {
            available_tasks.add(MoveHere {
                endpoint: **trigger.event(),
                movement: Movement::Walk,
            });
            available_tasks.add(MoveHere {
                endpoint: **trigger.event(),
                movement: Movement::Run,
            });
        }
    }

    fn start_navigation(
        mut actors: Query<(&mut NavSettings, &mut NavDestination)>,
        tasks: Query<(&Parent, &MoveHere, &TaskState), Changed<TaskState>>,
    ) {
        for (parent, move_here, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let (mut nav_settings, mut dest) = actors
                    .get_mut(**parent)
                    .expect("actors should have navigation component");
                *nav_settings = NavSettings::new(move_here.movement.speed());
                **dest = Some(move_here.endpoint);
            }
        }
    }

    fn finish(
        mut commands: Commands,
        actors: Query<(&Children, &NavDestination), Changed<NavDestination>>,
        tasks: Query<(Entity, &TaskState), With<MoveHere>>,
    ) {
        for (children, dest) in &actors {
            if dest.is_none() {
                if let Some((entity, _)) = tasks
                    .iter_many(children)
                    .find(|(_, &task_state)| task_state == TaskState::Active)
                {
                    commands.entity(entity).despawn();
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
struct MoveHere {
    endpoint: Vec3,
    movement: Movement,
}

impl Task for MoveHere {
    fn name(&self) -> &str {
        match self.movement {
            Movement::Walk => "Walk here",
            Movement::Run => "Move here",
        }
    }

    fn groups(&self) -> TaskGroups {
        TaskGroups::LEGS
    }
}
