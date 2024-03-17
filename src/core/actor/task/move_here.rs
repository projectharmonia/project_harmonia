use bevy::prelude::*;
use bevy_replicon::prelude::*;
use oxidized_navigation::{NavMesh, NavMeshSettings};
use serde::{Deserialize, Serialize};

use crate::core::{
    actor::{
        movement_animation::Movement,
        task::{Task, TaskGroups, TaskList, TaskListSet, TaskState},
    },
    city::Ground,
    cursor_hover::CursorHover,
    game_world::GameWorld,
    navigation::{ComputePath, NavPath, Navigation},
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MoveHere>()
            .replicate::<MoveHere>()
            .add_systems(
                Update,
                (Self::add_to_list.in_set(TaskListSet), Self::finish)
                    .run_if(resource_exists::<GameWorld>),
            )
            // Should run in `PostUpdate` to let tiles initialize.
            .add_systems(PostUpdate, Self::start_navigation.run_if(has_authority));
    }
}

impl MoveHerePlugin {
    fn add_to_list(
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

    fn start_navigation(
        mut commands: Commands,
        mut actors: Query<(&Transform, &mut Navigation)>,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        tasks: Query<(&Parent, &MoveHere, &TaskState), Changed<TaskState>>,
    ) {
        for (parent, move_here, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let (transform, mut navigation) = actors
                    .get_mut(**parent)
                    .expect("actors should have navigation component");
                *navigation = Navigation::new(move_here.movement.speed());
                commands.entity(**parent).insert(ComputePath::new(
                    nav_mesh.get(),
                    nav_settings.clone(),
                    transform.translation,
                    move_here.endpoint,
                ));
            }
        }
    }

    fn finish(
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
