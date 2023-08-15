use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    actor::{
        movement::Movement,
        task::{Task, TaskGroups, TaskList, TaskListSet, TaskState},
    },
    city::Ground,
    cursor_hover::CursorHover,
    game_world::WorldName,
    navigation::{endpoint::Endpoint, Navigation},
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<MoveHere>().add_systems(
            Update,
            (
                Self::list_system.in_set(TaskListSet),
                Self::activation_system,
                Self::cancellation_system,
                Self::finish_system,
            )
                .run_if(resource_exists::<WorldName>()),
        );
    }
}

impl MoveHerePlugin {
    fn list_system(
        mut list_events: EventWriter<TaskList>,
        mut grounds: Query<&CursorHover, With<Ground>>,
    ) {
        if let Ok(hover) = grounds.get_single_mut() {
            list_events.send(
                MoveHere {
                    endpoint: hover.0,
                    movement: Movement::Walk,
                }
                .into(),
            );
            list_events.send(
                MoveHere {
                    endpoint: hover.0,
                    movement: Movement::Run,
                }
                .into(),
            );
        }
    }

    fn activation_system(
        mut commands: Commands,
        tasks: Query<(&Parent, &MoveHere, &TaskState), Changed<TaskState>>,
    ) {
        for (parent, move_here, &state) in &tasks {
            if state == TaskState::Active {
                commands.entity(**parent).insert((
                    Navigation::new(move_here.movement.speed()),
                    Endpoint::new(move_here.endpoint),
                ));
            }
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        tasks: Query<(Entity, &Parent, &TaskState), Changed<TaskState>>,
    ) {
        for (entity, parent, &state) in &tasks {
            if state == TaskState::Cancelled {
                commands.entity(**parent).remove::<Navigation>();
                commands.entity(entity).despawn();
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
        children: Query<&Children>,
        tasks: Query<(Entity, &TaskState), With<MoveHere>>,
    ) {
        for children in children.iter_many(&mut removed_navigations) {
            if let Some((entity, _)) = tasks
                .iter_many(children)
                .find(|(_, &state)| state == TaskState::Active)
            {
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
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
