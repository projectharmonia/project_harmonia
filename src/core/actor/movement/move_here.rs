use bevy::{prelude::*, scene};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Movement, MovementBundle};
use crate::core::{
    actor::{
        task::{Task, TaskGroups, TaskList, TaskListSet, TaskState},
        ActorAnimation,
    },
    asset_handles::AssetHandles,
    cursor_hover::CursorHover,
    game_world::WorldName,
    ground::Ground,
    navigation::{endpoint::Endpoint, Navigation},
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<MoveHere>().add_systems(
            Update,
            (
                Self::list_system.in_set(TaskListSet),
                Self::activation_system.after(scene::scene_spawner_system),
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
                    MovementBundle::new(move_here.movement),
                    Endpoint::new(move_here.endpoint),
                ));
            }
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        tasks: Query<(&Parent, &TaskState), Changed<TaskState>>,
    ) {
        for (parent, &state) in &tasks {
            if state == TaskState::Cancelled {
                commands.entity(**parent).remove::<Navigation>();
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut removed_movements: RemovedComponents<Movement>,
        mut actors: Query<(&Children, &mut Handle<AnimationClip>)>,
        tasks: Query<(Entity, &TaskState), With<MoveHere>>,
    ) {
        for actor_entity in &mut removed_movements {
            if let Ok((children, mut animation_handle)) = actors.get_mut(actor_entity) {
                if let Some((task_entity, _)) = tasks
                    .iter_many(children)
                    .find(|(_, &state)| state != TaskState::Queued)
                {
                    commands.entity(task_entity).despawn();
                    *animation_handle = actor_animations.handle(ActorAnimation::Idle);
                }
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
