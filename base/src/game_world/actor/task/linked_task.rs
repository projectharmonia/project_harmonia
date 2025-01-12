use bevy::prelude::*;

pub(super) struct LinkedTaskPlugin;

impl Plugin for LinkedTaskPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::cleanup);
    }
}

impl LinkedTaskPlugin {
    fn cleanup(
        trigger: Trigger<OnRemove, LinkedTask>,
        mut commands: Commands,
        tasks: Query<&LinkedTask>,
    ) {
        let linked_task = tasks.get(trigger.entity()).unwrap();
        if let Some(mut entity) = linked_task.and_then(|entity| commands.get_entity(entity)) {
            entity.despawn();
        }
    }
}

/// Stores entity of another tasks and syncs [`TaskState`] between them.
///
/// If this task will be despawned, the linked task will be despawned as well.
#[derive(Component, Clone, Copy, Deref, DerefMut, Default)]
pub(super) struct LinkedTask(pub(super) Option<Entity>);
