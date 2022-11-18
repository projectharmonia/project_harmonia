mod despawn_tracker;
pub(crate) mod map_entity;
mod removal_tracker;
mod world_diff;

use bevy::{
    app::PluginGroupBuilder,
    ecs::{
        archetype::ArchetypeId,
        component::{ComponentId, ComponentTicks, StorageType},
    },
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryInternal},
    utils::HashMap,
};
use bevy_renet::renet::{RenetClient, RenetServer, ServerEvent};
use iyes_loopless::prelude::*;
use rmp_serde::Deserializer;
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use tap::TapFallible;

use super::server::ServerFixedTimestep;
use super::{client, REPLICATION_CHANNEL_ID};
use crate::core::{
    doll::DollPlayers,
    game_state::GameState,
    game_world::{save_rules::SaveRules, GameWorld},
};
use despawn_tracker::{DespawnTracker, DespawnTrackerPlugin};
use map_entity::{NetworkEntityMap, ReflectMapEntity};
use removal_tracker::{RemovalTracker, RemovalTrackerPlugin};
use world_diff::{ComponentDiff, WorldDiff, WorldDiffDeserializer, WorldDiffSerializer};

pub(super) struct ReplicationPlugins;

impl PluginGroup for ReplicationPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(RemovalTrackerPlugin)
            .add(DespawnTrackerPlugin)
            .add(ReplicationPlugin);
    }
}

/// Replicates components using [`SaveRules`] and [`NetworkComponents`] list from server to client.
struct ReplicationPlugin;

impl Plugin for ReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastTick>()
            .init_resource::<AckedTicks>()
            .init_resource::<NetworkEntityMap>()
            .init_resource::<NetworkComponents>()
            .add_system(
                Self::world_diff_receiving_system
                    .into_conditional_exclusive()
                    .run_if(client::connected)
                    .at_start(),
            )
            .add_system(Self::tick_ack_sending_system.run_if(client::connected))
            .add_system(Self::tick_acks_receiving_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::acked_ticks_cleanup_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::server_reset_system.run_if_resource_removed::<RenetServer>())
            .add_system(Self::client_reset_system.run_if_resource_removed::<RenetClient>());

        let world_diffs_sending_system =
            Self::world_diffs_sending_system.run_if_resource_exists::<RenetServer>();

        if cfg!(test) {
            app.add_system_to_stage(CoreStage::Update, world_diffs_sending_system);
        } else {
            app.add_fixed_timestep_system(
                ServerFixedTimestep::Tick.into(),
                0,
                world_diffs_sending_system,
            );
        }
    }
}

impl ReplicationPlugin {
    fn world_diffs_sending_system(
        mut set: ParamSet<(&World, ResMut<RenetServer>)>,
        acked_ticks: Res<AckedTicks>,
        registry: Res<TypeRegistry>,
        save_rules: Res<SaveRules>,
        network_components: Res<NetworkComponents>,
        despawn_tracker: Res<DespawnTracker>,
        removal_trackers: Query<(Entity, &RemovalTracker)>,
    ) {
        // Initialize [`WorldDiff`]s with latest acknowledged tick for each client.
        let mut client_diffs: HashMap<_, _> = acked_ticks
            .iter()
            .map(|(&client_id, &last_tick)| (client_id, WorldDiff::new(last_tick)))
            .collect();
        collect_changes(
            &mut client_diffs,
            set.p0(),
            &registry,
            &save_rules,
            &network_components,
        );
        collect_removals(&mut client_diffs, set.p0(), &removal_trackers);
        collect_despawns(&mut client_diffs, &despawn_tracker);

        let current_tick = set.p0().read_change_tick();
        let registry = registry.read();
        for (client_id, mut world_diff) in client_diffs {
            world_diff.tick = current_tick; // Replace last acknowledged tick with the current.
            let serializer = WorldDiffSerializer::new(&world_diff, &registry);
            let message = rmp_serde::to_vec(&serializer).expect("world diff should be serialized");
            set.p1()
                .send_message(client_id, REPLICATION_CHANNEL_ID, message);
        }
    }

    fn world_diff_receiving_system(world: &mut World) {
        world.resource_scope(|world, registry: Mut<TypeRegistry>| {
            if let Some(world_diff) =
                receive_world_diff(&mut world.resource_mut::<RenetClient>(), &registry.read())
            {
                let mut last_tick = world.resource_mut::<LastTick>();
                if last_tick.0 < world_diff.tick {
                    last_tick.0 = world_diff.tick;
                }

                world.resource_scope(|world, mut entity_map: Mut<NetworkEntityMap>| {
                    apply_diffs(world, &mut entity_map, &registry, world_diff);
                });
                if !world.contains_resource::<GameWorld>() {
                    world.insert_resource(GameWorld::default()); // TODO: Replicate this resource.
                    world.insert_resource(NextState(GameState::World));
                }
            }
        });
    }

    fn tick_ack_sending_system(last_tick: Res<LastTick>, mut client: ResMut<RenetClient>) {
        let message = rmp_serde::to_vec(&*last_tick)
            .unwrap_or_else(|e| panic!("client ack should be serialized: {e}"));
        client.send_message(REPLICATION_CHANNEL_ID, message);
    }

    fn tick_acks_receiving_system(
        mut acked_ticks: ResMut<AckedTicks>,
        mut server: ResMut<RenetServer>,
    ) {
        for client_id in server.clients_id() {
            if let Some(tick) = receive_tick_ack(&mut server, client_id) {
                let last_tick = acked_ticks.entry(client_id).or_default();
                if *last_tick < tick.0 {
                    *last_tick = tick.0;
                }
            }
        }
    }

    fn acked_ticks_cleanup_system(
        mut server_events: EventReader<ServerEvent>,
        mut acked_ticks: ResMut<AckedTicks>,
    ) {
        for event in server_events.iter() {
            if let ServerEvent::ClientDisconnected(id) = event {
                acked_ticks.remove(id);
            }
        }
    }

    fn server_reset_system(mut commands: Commands) {
        commands.insert_resource(AckedTicks::default());
    }

    fn client_reset_system(mut commands: Commands) {
        commands.insert_resource(LastTick::default());
        commands.insert_resource(NetworkEntityMap::default());
    }
}

/// Reads all world diff from socket and returns the latest if it was received.
fn receive_world_diff(
    client: &mut RenetClient,
    registry: &TypeRegistryInternal,
) -> Option<WorldDiff> {
    let mut received_diffs = Vec::<WorldDiff>::new();
    while let Some(message) = client.receive_message(REPLICATION_CHANNEL_ID) {
        let mut deserializer = Deserializer::from_read_ref(&message);
        let world_diff = WorldDiffDeserializer::new(registry)
            .deserialize(&mut deserializer)
            .expect("server should send valid world diffs");
        received_diffs.push(world_diff);
    }

    received_diffs
        .into_iter()
        .max_by_key(|world_diff| world_diff.tick)
}

fn receive_tick_ack(server: &mut RenetServer, client_id: u64) -> Option<LastTick> {
    let mut received_ticks = Vec::<LastTick>::new();
    while let Some(message) = server.receive_message(client_id, REPLICATION_CHANNEL_ID) {
        if let Ok(tick) = rmp_serde::from_slice(&message)
            .tap_err(|e| error!("unable to deserialize a tick from client {client_id}: {e}"))
        {
            received_ticks.push(tick);
        }
    }

    received_ticks.into_iter().max_by_key(|tick| tick.0)
}

fn collect_changes(
    client_diffs: &mut HashMap<u64, WorldDiff>,
    world: &World,
    registry: &TypeRegistry,
    save_rules: &SaveRules,
    network_components: &NetworkComponents,
) {
    let registry = registry.read();
    for archetype in world
        .archetypes()
        .iter()
        .filter(|archetype| archetype.id() != ArchetypeId::EMPTY)
        .filter(|archetype| archetype.id() != ArchetypeId::RESOURCE)
        .filter(|archetype| archetype.id() != ArchetypeId::INVALID)
        .filter(|archetype| save_rules.persistent_archetype(archetype))
    {
        let table = world
            .storages()
            .tables
            .get(archetype.table_id())
            .expect("archetype should have a table");

        for component_id in archetype.components().filter(|&component_id| {
            save_rules.persistent_component(archetype, component_id)
                || network_components.contains(&component_id)
        }) {
            let storage_type = archetype
                .get_storage_type(component_id)
                .expect("archetype should have a storage type");

            let component_info = unsafe { world.components().get_info_unchecked(component_id) };
            let type_name = component_info.name();
            let reflect_component = component_info
                .type_id()
                .and_then(|type_id| registry.get(type_id))
                .and_then(|registration| registration.data::<ReflectComponent>())
                .unwrap_or_else(|| panic!("non-ignored component {type_name} should be registered and have reflect(Component)"));

            match storage_type {
                StorageType::Table => {
                    let column = table
                        .get_column(component_id)
                        .unwrap_or_else(|| panic!("{type_name} should have a valid column"));

                    for entity in archetype.entities() {
                        let location = world
                            .entities()
                            .get(*entity)
                            .expect("entity exist in archetype table");
                        let table_row = archetype.entity_table_row(location.index);
                        // Safe: the table row is obtained safely from the world's state
                        let ticks = unsafe { &*column.get_ticks_unchecked(table_row).get() };
                        collect_if_changed(
                            client_diffs,
                            *entity,
                            world,
                            ticks,
                            reflect_component,
                            type_name,
                        );
                    }
                }
                StorageType::SparseSet => {
                    let sparse_set = world
                        .storages()
                        .sparse_sets
                        .get(component_id)
                        .unwrap_or_else(|| panic!("{type_name} should exists in a sparse set"));

                    for entity in archetype.entities() {
                        let ticks = unsafe {
                            &*sparse_set
                                .get_ticks(*entity)
                                .expect("{type_name} should have ticks")
                                .get()
                        };
                        collect_if_changed(
                            client_diffs,
                            *entity,
                            world,
                            ticks,
                            reflect_component,
                            type_name,
                        );
                    }
                }
            }
        }
    }
}

fn collect_if_changed(
    client_diffs: &mut HashMap<u64, WorldDiff>,
    entity: Entity,
    world: &World,
    ticks: &ComponentTicks,
    reflect_component: &ReflectComponent,
    type_name: &str,
) {
    for world_diff in client_diffs.values_mut() {
        if ticks.is_changed(world_diff.tick, world.read_change_tick()) {
            let component = reflect_component
                .reflect(world, entity)
                .unwrap_or_else(|| panic!("entity should have {type_name}"))
                .clone_value();
            world_diff
                .entities
                .entry(entity)
                .or_default()
                .push(ComponentDiff::Changed(component));
        }
    }
}

fn collect_removals(
    client_diffs: &mut HashMap<u64, WorldDiff>,
    world: &World,
    removal_trackers: &Query<(Entity, &RemovalTracker)>,
) {
    for (entity, removal_tracker) in removal_trackers {
        for world_diff in client_diffs.values_mut() {
            for (&component_id, &tick) in removal_tracker.iter() {
                if world_diff.tick < tick {
                    let component_info =
                        unsafe { world.components().get_info_unchecked(component_id) };
                    world_diff
                        .entities
                        .entry(entity)
                        .or_default()
                        .push(ComponentDiff::Removed(component_info.name().to_string()));
                }
            }
        }
    }
}

fn collect_despawns(client_diffs: &mut HashMap<u64, WorldDiff>, despawn_tracker: &DespawnTracker) {
    for (entity, tick) in despawn_tracker.despawns.iter().copied() {
        for world_diff in client_diffs.values_mut() {
            if world_diff.tick < tick {
                world_diff.despawns.push(entity);
            }
        }
    }
}

fn apply_diffs(
    world: &mut World,
    entity_map: &mut NetworkEntityMap,
    registry: &TypeRegistry,
    world_diff: WorldDiff,
) {
    let registry = registry.read();
    // Map entities non-lazily in order to correctly map components that reference server entities.
    for (entity, components) in map_entities(world, entity_map, world_diff.entities) {
        for component_diff in components {
            apply_component_diff(world, entity_map, &registry, entity, &component_diff);
        }
    }
    for server_entity in world_diff.despawns {
        let local_entity = entity_map
            .remove_by_server(server_entity)
            .expect("server should send valid entities to despawn");
        world.entity_mut(local_entity).despawn_recursive();
    }
}

/// Maps entities received from server into local entities.
fn map_entities(
    world: &mut World,
    entity_map: &mut NetworkEntityMap,
    entities: HashMap<Entity, Vec<ComponentDiff>>,
) -> Vec<(Entity, Vec<ComponentDiff>)> {
    let mut mapped_entities = Vec::with_capacity(entities.len());
    for (server_entity, components) in entities {
        let client_entity = entity_map.get_by_server_or_spawn(world, server_entity);
        mapped_entities.push((client_entity, components));
    }
    mapped_entities
}

fn apply_component_diff(
    world: &mut World,
    entity_map: &NetworkEntityMap,
    registry: &TypeRegistryInternal,
    local_entity: Entity,
    component_diff: &ComponentDiff,
) {
    let type_name = component_diff.type_name();
    let registration = registry
        .get_with_name(type_name)
        .unwrap_or_else(|| panic!("{type_name} should be registered"));

    let reflect_component = registration
        .data::<ReflectComponent>()
        .unwrap_or_else(|| panic!("{type_name} should have reflect(Component)"));

    match component_diff {
        ComponentDiff::Changed(component) => {
            reflect_component.apply_or_insert(world, local_entity, &**component);
            if let Some(reflect_map_entities) = registration.data::<ReflectMapEntity>() {
                reflect_map_entities
                    .map_entities(world, entity_map.to_client(), local_entity)
                    .unwrap_or_else(|e| panic!("entities in {type_name} should be mappable: {e}"));
            }
        }
        ComponentDiff::Removed(_) => reflect_component.remove(world, local_entity),
    }
}

/// Last received tick from server.
///
/// Used only on clients.
#[derive(Default, Serialize, Deserialize)]
struct LastTick(u32);

/// Last acknowledged server ticks from all clients.
///
/// Used only on server.
#[derive(Default, Deref, DerefMut)]
struct AckedTicks(HashMap<u64, u32>);

/// List of component IDs that should be replicated.
///
/// This list contains only components that are not in [`SaveRules`]
/// (i.e. non-persistent components), but that should be networked.
#[derive(Deref, DerefMut)]
struct NetworkComponents(Vec<ComponentId>);

impl FromWorld for NetworkComponents {
    fn from_world(world: &mut World) -> Self {
        Self(vec![world.init_component::<DollPlayers>()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        game_world::{parent_sync::ParentSync, GameEntity},
        network::{network_preset::NetworkPresetPlugin, replication::map_entity::NetworkEntityMap},
    };

    #[test]
    fn acked_ticks_cleanup() {
        let mut app = App::new();
        app.add_plugin(TestReplicationPlugin);

        let mut client = app.world.resource_mut::<RenetClient>();
        client.disconnect();
        let client_id = client.client_id();

        let mut acked_ticks = app.world.resource_mut::<AckedTicks>();
        acked_ticks.insert(client_id, 0);

        app.update();

        let acked_ticks = app.world.resource::<AckedTicks>();
        assert!(!acked_ticks.contains_key(&client_id));
    }

    #[test]
    fn tick_acks_receiving() {
        let mut app = App::new();
        app.add_plugin(TestReplicationPlugin);

        for _ in 0..5 {
            app.update();
        }

        let acked_ticks = app.world.resource::<AckedTicks>();
        let client = app.world.resource::<RenetClient>();
        assert!(matches!(acked_ticks.get(&client.client_id()), Some(&last_tick) if last_tick > 0));
    }

    #[test]
    fn spawn_replication() {
        let mut app = App::new();
        app.register_type::<GameEntity>()
            .add_plugin(TestReplicationPlugin);

        // Wait two ticks to send and receive acknowledge.
        app.update();
        app.update();

        let server_entity = app.world.spawn().insert(GameEntity).id();

        app.update();

        // Remove server entity before client replicates it,
        // since in test client and server in the same world.
        app.world.entity_mut(server_entity).despawn();

        app.update();

        let client_entity = app
            .world
            .query::<Entity>()
            .get_single(&app.world)
            .expect("server entity should be replicated to client");
        let entity_map = app.world.resource::<NetworkEntityMap>();
        let mapped_entity = entity_map
            .to_client()
            .get(server_entity)
            .expect("server entity should be mapped on client");
        assert_eq!(
            mapped_entity, client_entity,
            "mapped entity should correspond to the replicated entity on client"
        );
        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::World
        );
    }

    #[test]
    fn change_replicaiton() {
        let mut app = App::new();
        app.register_type::<GameEntity>()
            .register_type::<SparseSetComponent>()
            .add_plugin(TestReplicationPlugin);

        app.update();
        app.update();

        app.world
            .resource_scope(|world, mut save_rules: Mut<NetworkComponents>| {
                save_rules.push(world.init_component::<SparseSetComponent>());
            });

        let replicated_entity = app
            .world
            .spawn()
            .insert(GameEntity)
            .insert(SparseSetComponent)
            .insert(NonReflectedComponent)
            .id();

        // Mark as already spawned.
        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(replicated_entity, replicated_entity);

        app.update();

        // Remove components before client replicates it,
        // since in test client and server in the same world.
        let mut replicated_entity = app.world.entity_mut(replicated_entity);
        replicated_entity.remove::<GameEntity>();
        replicated_entity.remove::<SparseSetComponent>();
        replicated_entity.remove::<NonReflectedComponent>();
        let replicated_entity = replicated_entity.id();

        app.update();

        let replicated_entity = app.world.entity(replicated_entity);
        assert!(replicated_entity.contains::<GameEntity>());
        assert!(replicated_entity.contains::<SparseSetComponent>());
        assert!(!replicated_entity.contains::<NonReflectedComponent>());
    }

    #[test]
    fn entity_mapping() {
        let mut app = App::new();
        app.register_type::<GameEntity>()
            .register_type::<ParentSync>()
            .add_plugin(TestReplicationPlugin);

        app.update();
        app.update();

        let client_parent = app.world.spawn().id();
        let server_parent = app.world.spawn().id();
        let replicated_entity = app
            .world
            .spawn()
            .insert(GameEntity)
            .insert(ParentSync(server_parent))
            .id();

        let mut entity_map = app.world.resource_mut::<NetworkEntityMap>();
        entity_map.insert(replicated_entity, replicated_entity);
        entity_map.insert(server_parent, client_parent);

        app.update();
        app.update();

        let parent_sync = app.world.get::<ParentSync>(replicated_entity).unwrap();
        assert_eq!(parent_sync.0, client_parent);
    }

    #[test]
    fn removal_replication() {
        let mut app = App::new();
        app.register_type::<NonReflectedComponent>()
            .register_type::<GameEntity>()
            .add_plugin(TestReplicationPlugin);

        app.update();
        app.update();

        // Mark components as removed.
        const REMOVAL_TICK: u32 = 1; // Should be more then 0 since both client and server starts with 0 tick and think that everything is replicated at this point.
        let game_entity_id = app.world.init_component::<GameEntity>();
        let removal_tracker = RemovalTracker(HashMap::from([(game_entity_id, REMOVAL_TICK)]));
        let replicated_entity = app
            .world
            .spawn()
            .insert(removal_tracker)
            .insert(GameEntity)
            .insert(NonReflectedComponent)
            .id();

        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(replicated_entity, replicated_entity);

        app.update();
        app.update();

        let replicated_entity = app.world.entity(replicated_entity);
        assert!(!replicated_entity.contains::<GameEntity>());
        assert!(replicated_entity.contains::<NonReflectedComponent>());
    }

    #[test]
    fn despawn_replication() {
        let mut app = App::new();
        app.add_plugin(TestReplicationPlugin);

        app.update();
        app.update();

        let children_entity = app.world.spawn().id();
        let despawned_entity = app.world.spawn().push_children(&[children_entity]).id();
        let current_tick = app.world.read_change_tick();
        let mut despawn_tracker = app.world.resource_mut::<DespawnTracker>();
        despawn_tracker
            .despawns
            .push((despawned_entity, current_tick));

        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(despawned_entity, despawned_entity);

        app.update();
        app.update();

        assert!(app.world.get_entity(despawned_entity).is_none());
        assert!(app.world.get_entity(children_entity).is_none());
        assert!(app
            .world
            .resource::<NetworkEntityMap>()
            .to_client()
            .is_empty());
    }

    struct TestReplicationPlugin;

    impl Plugin for TestReplicationPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<SaveRules>()
                .init_resource::<DespawnTracker>()
                .add_plugin(NetworkPresetPlugin::client_and_server())
                .add_plugin(ReplicationPlugin);
        }
    }

    #[derive(Component, Default, Reflect)]
    #[component(storage = "SparseSet")]
    #[reflect(Component)]
    struct SparseSetComponent;

    #[derive(Component, Reflect)]
    struct NonReflectedComponent;
}
