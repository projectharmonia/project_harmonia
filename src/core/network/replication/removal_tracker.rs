use bevy::{ecs::component::ComponentId, prelude::*, utils::HashMap};
use bevy_renet::renet::RenetServer;
use iyes_loopless::prelude::IntoConditionalSystem;

use super::AckedTicks;
use crate::core::game_world::{ignore_rules::IgnoreRules, GameEntity};

/// Stores component removals in [`RemovalTracker`] component
/// to make them persistent across ticks.
pub(super) struct RemovalTrackerPlugin;

impl Plugin for RemovalTrackerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::insertion_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::cleanup_system.run_if_resource_exists::<RenetServer>())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::detection_system.run_if_resource_exists::<RenetServer>(),
            );
    }
}

impl RemovalTrackerPlugin {
    fn insertion_system(
        mut commands: Commands,
        new_game_entities: Query<Entity, (Added<GameEntity>, Without<RemovalTracker>)>,
    ) {
        for entity in &new_game_entities {
            commands.entity(entity).insert(RemovalTracker::default());
        }
    }

    fn cleanup_system(
        client_acks: Res<AckedTicks>,
        mut removal_trackers: Query<&mut RemovalTracker>,
    ) {
        for mut removal_tracker in &mut removal_trackers {
            removal_tracker
                .retain(|_, tick| client_acks.values().any(|last_tick| last_tick < tick));
        }
    }

    fn detection_system(
        mut set: ParamSet<(&World, Query<&mut RemovalTracker>)>,
        ignore_rules: Res<IgnoreRules>,
    ) {
        let current_tick = set.p0().read_change_tick();
        for component_id in ignore_rules.serializable.iter().copied() {
            let entities: Vec<_> = set.p0().removed_with_id(component_id).collect();
            for entity in entities {
                if let Ok(mut removal_tracker) = set.p1().get_mut(entity) {
                    removal_tracker.insert(component_id, current_tick);
                }
            }
        }
    }
}

#[derive(Component, Default, Deref, DerefMut)]
pub(super) struct RemovalTracker(pub(super) HashMap<ComponentId, u32>);

#[cfg(test)]
mod tests {
    use crate::core::network::network_preset::NetworkPresetPlugin;

    use super::*;

    #[test]
    fn insertion() {
        let mut app = App::new();
        app.add_plugin(TestRemovalTrackerPlugin);

        let game_entity = app.world.spawn().insert(GameEntity).id();

        app.update();

        assert!(app.world.entity(game_entity).contains::<RemovalTracker>());
    }

    #[test]
    fn cleanup() {
        let mut app = App::new();
        app.add_plugin(TestRemovalTrackerPlugin);

        let current_tick = app.world.read_change_tick();
        const COMPONENT_ID: ComponentId = ComponentId::new(0);
        let removal_tracker = RemovalTracker(HashMap::from([(COMPONENT_ID, current_tick)]));
        let game_entity = app.world.spawn().insert(removal_tracker).id();

        const DUMMY_CLIENT_ID: u64 = 0;
        app.world
            .resource_mut::<AckedTicks>()
            .insert(DUMMY_CLIENT_ID, current_tick);

        app.update();

        let removal_tracker = app.world.get::<RemovalTracker>(game_entity).unwrap();
        assert!(!removal_tracker.contains_key(&COMPONENT_ID));
        assert!(removal_tracker.is_empty());
    }

    #[test]
    fn detection() {
        let mut app = App::new();
        app.add_plugin(TestRemovalTrackerPlugin);

        let game_entity = app
            .world
            .spawn()
            .insert(GameEntity)
            .insert(Transform::default())
            .insert(RemovalTracker::default())
            .id();

        app.world.entity_mut(game_entity).remove::<Transform>();

        // A non-trackable entity.
        app.world
            .spawn()
            .insert(Transform::default())
            .remove::<Transform>();

        app.update();

        let transform_id = app.world.component_id::<Transform>().unwrap();
        let removal_tracker = app.world.get::<RemovalTracker>(game_entity).unwrap();
        assert!(removal_tracker.contains_key(&transform_id));
        assert_eq!(removal_tracker.len(), 1);
    }

    struct TestRemovalTrackerPlugin;

    impl Plugin for TestRemovalTrackerPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<AckedTicks>()
                .init_resource::<IgnoreRules>()
                .add_plugin(NetworkPresetPlugin::server())
                .add_plugin(RemovalTrackerPlugin);
        }
    }
}
