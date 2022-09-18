use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::GameWorld;

pub(super) struct ParentSyncPlugin;

/// Automatically updates hierarchy when [`SyncParent`] is changed.
///
/// This allows to save / replicate hierarchy using only [`SyncParent`] component.
impl Plugin for ParentSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::parent_sync_system.run_if_resource_exists::<GameWorld>());
    }
}

impl ParentSyncPlugin {
    fn parent_sync_system(
        mut commands: Commands,
        changed_parents: Query<(Entity, &ParentSync), Changed<ParentSync>>,
    ) {
        for (entity, parent) in &changed_parents {
            commands.entity(parent.0).push_children(&[entity]);
        }
    }
}

#[derive(Component)]
pub(crate) struct ParentSync(Entity);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_sync() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_plugin(ParentSyncPlugin);

        let parent = app.world.spawn().id();
        let child = app.world.spawn().insert(ParentSync(parent)).id();

        app.update();

        let child_parent = app.world.get::<Parent>(child).unwrap();
        assert_eq!(parent, child_parent.get());

        let children = app.world.get::<Children>(parent).unwrap();
        assert!(children.contains(&child));
    }
}
