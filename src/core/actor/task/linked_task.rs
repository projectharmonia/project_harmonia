use bevy::prelude::*;

use super::TaskState;
use crate::core::animation_state::AnimationState;

pub(super) struct LinkedTaskPlugin;

impl Plugin for LinkedTaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                Self::link_system,
                Self::state_sync_system,
                Self::finish_system,
            ),
        );
    }
}

impl LinkedTaskPlugin {
    fn link_system(mut commands: Commands, tasks: Query<(Entity, &LinkedTask), Added<LinkedTask>>) {
        for (entity, linked_task) in &tasks {
            commands.entity(linked_task.0).insert(LinkedTask(entity));
        }
    }

    fn state_sync_system(tasks: Query<(&mut TaskState, &LinkedTask), Changed<TaskState>>) {
        for (&task_state, &linked_task) in &tasks {
            // SAFETY: called only on linked entities, one at a time.
            if let Ok(mut linked_state) =
                unsafe { tasks.get_component_unchecked_mut::<TaskState>(linked_task.0) }
            {
                if *linked_state != task_state {
                    *linked_state = task_state;
                }
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut removed_tasks: RemovedComponents<LinkedTask>,
        tasks: Query<(Entity, &Parent, &TaskState, &LinkedTask)>,
        mut actors: Query<&mut AnimationState>,
    ) {
        for removed_entity in &mut removed_tasks {
            if let Some((linked_entity, parent, &task_state, _)) = tasks
                .iter()
                .find(|(.., linked_task)| linked_task.0 == removed_entity)
            {
                if task_state == TaskState::Active {
                    let mut animation_state = actors
                        .get_mut(**parent)
                        .expect("actor should have animaition state");
                    animation_state.stop();

                    commands.entity(linked_entity).despawn();
                }
            }
        }
    }
}

/// Stores entity of another tasks and syncs [`TaskState`] between them.
///
/// Same component will be automatically added to the linked task too.
/// After insertion current task state changes to the linked state.
/// Current task will be considered finished after linked task despawn.
#[derive(Clone, Component, Copy)]
pub(super) struct LinkedTask(pub(super) Entity);
