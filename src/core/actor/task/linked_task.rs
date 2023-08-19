use bevy::prelude::*;

use super::TaskState;

pub(super) struct LinkedTaskPlugin;

impl Plugin for LinkedTaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, (Self::link_system, Self::state_sync_system));
    }
}

impl LinkedTaskPlugin {
    fn link_system(mut commands: Commands, tasks: Query<(Entity, &LinkedTask), Added<LinkedTask>>) {
        for (entity, link) in &tasks {
            commands.entity(link.0).insert(LinkedTask(entity));
        }
    }

    fn state_sync_system(tasks: Query<(&mut TaskState, &LinkedTask), Changed<TaskState>>) {
        for (&state, &link) in &tasks {
            // SAFETY: called only on linked entities, one at a time.
            if let Ok(mut linked_state) =
                unsafe { tasks.get_component_unchecked_mut::<TaskState>(link.0) }
            {
                if *linked_state != state {
                    *linked_state = state;
                }
            }
        }
    }
}

/// Stores entity of another tasks and syncs [`TaskState`] between them.
#[derive(Clone, Component, Copy)]
pub(super) struct LinkedTask(pub(super) Entity);
