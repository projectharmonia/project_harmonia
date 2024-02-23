use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    actor::{
        movement_animation::Movement,
        task::{Task, TaskGroups, TaskList, TaskListSet, TaskState},
    },
    city::Ground,
    cursor_hover::CursorHover,
    game_world::WorldName,
    navigation::{endpoint::Endpoint, NavPath, Navigation},
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveHere>()
            .replicate::<MoveHere>()
            .add_systems(
                Update,
                (
                    Self::list_system.in_set(TaskListSet),
                    Self::activation_system,
                    Self::finish_system,
                )
                    .run_if(resource_exists::<WorldName>),
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
        mut actors: Query<&mut Navigation>,
        tasks: Query<(&Parent, &MoveHere, &TaskState), Changed<TaskState>>,
    ) {
        for (parent, move_here, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let mut navigation = actors
                    .get_mut(**parent)
                    .expect("actors should have navigation component");
                *navigation = Navigation::new(move_here.movement.speed());
                commands
                    .entity(**parent)
                    .insert(Endpoint::new(move_here.endpoint));
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        actors: Query<(&Children, &NavPath), Changed<NavPath>>,
        tasks: Query<(Entity, &TaskState), With<MoveHere>>,
    ) {
        for (children, nav_path) in &actors {
            if nav_path.is_empty() {
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
