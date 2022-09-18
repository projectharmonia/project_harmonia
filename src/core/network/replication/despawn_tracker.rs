use bevy::{ecs::system::SystemChangeTick, prelude::*, utils::HashSet};
use bevy_renet::renet::RenetServer;
use iyes_loopless::prelude::IntoConditionalSystem;

use super::AckedTicks;
use crate::core::game_world::GameEntity;

/// Tracks entity despawns in [`DespawnTracker`] resource.
pub(super) struct DespawnTrackerPlugin;

impl Plugin for DespawnTrackerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DespawnTracker>()
            .add_system(Self::entity_tracking_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::cleanup_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::detection_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::reset_system.run_if_resource_removed::<RenetServer>());
    }
}

impl DespawnTrackerPlugin {
    fn entity_tracking_system(
        mut tracker: ResMut<DespawnTracker>,
        new_game_entities: Query<Entity, Added<GameEntity>>,
    ) {
        for entity in &new_game_entities {
            tracker.tracked_entities.insert(entity);
        }
    }

    fn cleanup_system(mut despawn_tracker: ResMut<DespawnTracker>, client_acks: Res<AckedTicks>) {
        despawn_tracker
            .despawns
            .retain(|(_, tick)| client_acks.values().any(|last_tick| last_tick < tick));
    }

    fn detection_system(
        change_tick: SystemChangeTick,
        mut tracker: ResMut<DespawnTracker>,
        entities: Query<Entity>,
    ) {
        let DespawnTracker {
            ref mut tracked_entities,
            ref mut despawns,
        } = *tracker;

        tracked_entities.retain(|&entity| {
            if entities.get(entity).is_err() {
                despawns.push((entity, change_tick.change_tick()));
                false
            } else {
                true
            }
        });
    }

    fn reset_system(mut tracker: ResMut<DespawnTracker>) {
        tracker.tracked_entities.clear();
        tracker.despawns.clear();
    }
}

#[derive(Default)]
pub(super) struct DespawnTracker {
    tracked_entities: HashSet<Entity>,
    /// Entities and ticks when they were despawned.
    pub(super) despawns: Vec<(Entity, u32)>,
}

#[cfg(test)]
mod tests {
    use crate::core::network::tests::{NetworkPreset, TestNetworkPlugin};

    use super::*;

    #[test]
    fn entity_tracking() {
        let mut app = App::new();
        app.add_plugin(TestDespawnTrackerPlugin);

        let game_entity = app.world.spawn().insert(GameEntity).id();

        app.update();

        let despawn_tracker = app.world.resource::<DespawnTracker>();
        assert!(despawn_tracker.tracked_entities.contains(&game_entity));
    }

    #[test]
    fn cleanup() {
        let mut app = App::new();
        app.add_plugin(TestDespawnTrackerPlugin);

        let current_tick = app.world.read_change_tick();
        let removed_entity = Entity::from_raw(0);
        let mut despawn_tracker = app.world.resource_mut::<DespawnTracker>();
        despawn_tracker
            .despawns
            .push((removed_entity, current_tick));

        const DUMMY_CLIENT_ID: u64 = 0;
        app.world
            .resource_mut::<AckedTicks>()
            .insert(DUMMY_CLIENT_ID, current_tick);

        app.update();

        let despawn_tracker = app.world.resource::<DespawnTracker>();
        assert!(despawn_tracker.despawns.is_empty())
    }

    #[test]
    fn detection() {
        let mut app = App::new();
        app.add_plugin(TestDespawnTrackerPlugin);

        let existing_entity = app.world.spawn().id();
        let removed_entity = Entity::from_raw(existing_entity.id() + 1);
        let mut despawn_tracker = app.world.resource_mut::<DespawnTracker>();
        despawn_tracker.tracked_entities.insert(existing_entity);
        despawn_tracker.tracked_entities.insert(removed_entity);

        // To avoid cleanup. Removal tick will be greater.
        const DUMMY_CLIENT_ID: u64 = 0;
        let current_tick = app.world.read_change_tick();
        app.world
            .resource_mut::<AckedTicks>()
            .insert(DUMMY_CLIENT_ID, current_tick);

        app.update();

        let despawn_tracker = app.world.resource::<DespawnTracker>();
        assert!(despawn_tracker
            .despawns
            .iter()
            .any(|(entity, _)| *entity == removed_entity));
    }

    #[test]
    fn reset() {
        let mut app = App::new();
        app.add_plugin(TestDespawnTrackerPlugin);

        app.update();

        let dummy_entity = Entity::from_raw(0);
        const DUMMY_TICK: u32 = 0;
        let mut despawn_tracker = app.world.resource_mut::<DespawnTracker>();
        despawn_tracker.despawns.push((dummy_entity, DUMMY_TICK));
        despawn_tracker.tracked_entities.insert(dummy_entity);

        app.world.remove_resource::<RenetServer>();

        app.update();

        let despawn_tracker = app.world.resource::<DespawnTracker>();
        assert!(despawn_tracker.despawns.is_empty());
        assert!(despawn_tracker.tracked_entities.is_empty());
    }

    struct TestDespawnTrackerPlugin;

    impl Plugin for TestDespawnTrackerPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<AckedTicks>()
                .add_plugin(TestNetworkPlugin::new(NetworkPreset::Server))
                .add_plugin(DespawnTrackerPlugin);
        }
    }
}
