mod removal_tracker;
mod world_diff;

use std::time::Duration;

use bevy::{
    ecs::{
        archetype::ArchetypeId,
        component::{ComponentTicks, StorageType},
        entity::EntityMap,
    },
    prelude::*,
    reflect::TypeRegistry,
    utils::HashMap,
};
use bevy_renet::renet::{RenetClient, RenetServer, ServerEvent};
use iyes_loopless::prelude::*;
use rmp_serde::Deserializer;
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use tap::{TapFallible, TapOptional};

use super::Channel;
use crate::core::game_world::ignore_rules::IgnoreRules;
use world_diff::{ComponentDiff, WorldDiff, WorldDiffDeserializer, WorldDiffSerializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, StageLabel)]
enum ReplicationStage {
    Tick,
}

struct ReplicationPlugin;

impl Plugin for ReplicationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TickAck>()
            .init_resource::<ClientAcks>()
            .init_resource::<NetworkEntityMap>()
            .add_stage_before(
                CoreStage::Update,
                ReplicationStage::Tick,
                FixedTimestepStage::new(Duration::from_secs_f64(Self::TIMESTEP)).with_stage(
                    SystemStage::single(
                        Self::send_server_message_system.run_if_resource_exists::<RenetServer>(),
                    ),
                ),
            )
            .add_system(Self::client_acks_cleanup_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::receive_client_ack_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::send_client_ack_system.run_if_resource_exists::<RenetClient>())
            .add_system(Self::server_reset_system.run_if_resource_removed::<RenetServer>())
            .add_system(Self::client_reset_system.run_if_resource_removed::<RenetClient>());

        {
            // To avoid ambiguity: https://github.com/IyesGames/iyes_loopless/issues/15
            use iyes_loopless::condition::IntoConditionalExclusiveSystem;
            app.add_system(
                Self::receive_server_message_system
                    .run_if_resource_exists::<RenetClient>()
                    .at_start(),
            );
        }
    }
}

impl ReplicationPlugin {
    const TIMESTEP: f64 = 0.1;

    fn client_acks_cleanup_system(
        mut server_events: EventReader<ServerEvent>,
        mut client_acks: ResMut<ClientAcks>,
    ) {
        for event in server_events.iter() {
            if let ServerEvent::ClientDisconnected(id) = event {
                client_acks.remove(id);
            }
        }
    }

    fn receive_client_ack_system(
        mut client_acks: ResMut<ClientAcks>,
        mut server: ResMut<RenetServer>,
    ) {
        for client_id in server.clients_id() {
            let mut received_acks = Vec::<TickAck>::new();
            while let Some(message) = server.receive_message(client_id, Channel::Replication as u8)
            {
                if let Ok(tick_ack) = rmp_serde::from_slice(&message)
                    .tap_err(|e| error!("unable to deserialize message from client: {e:#?}"))
                {
                    received_acks.push(tick_ack);
                }
            }

            if let Some(tick_ack) = received_acks.iter().max_by_key(|tick_ack| tick_ack.0) {
                let last_ack = client_acks.entry(client_id).or_default();
                if *last_ack < tick_ack.0 {
                    *last_ack = tick_ack.0;
                }
            }
        }
    }

    fn send_server_message_system(
        mut set: ParamSet<(&World, ResMut<RenetServer>)>,
        client_acks: Res<ClientAcks>,
        registry: Res<TypeRegistry>,
        ignore_rules: Res<IgnoreRules>,
    ) {
        for (client_id, world_diff) in
            client_diffs(set.p0(), &registry, &ignore_rules, &client_acks)
        {
            let serializer = WorldDiffSerializer::new(&registry, &world_diff);
            let message = rmp_serde::to_vec(&serializer).expect("world diff should be serialized");
            set.p1()
                .send_message(client_id, Channel::Replication as u8, message);
        }
    }

    fn receive_server_message_system(world: &mut World) {
        world.resource_scope(|world, registry: Mut<TypeRegistry>| {
            let mut received_diffs = Vec::<WorldDiff>::new();
            let mut client = world.resource_mut::<RenetClient>();
            while let Some(message) = client.receive_message(Channel::Replication as u8) {
                let mut deserializer = Deserializer::from_read_ref(&message);
                if let Ok(world_diff) = WorldDiffDeserializer::new(&registry)
                    .deserialize(&mut deserializer)
                    .tap_err(|e| error!("unable to deserialize server world diff: {e}"))
                {
                    received_diffs.push(world_diff);
                }
            }

            let mut tick_ack = world.resource_mut::<TickAck>();
            let world_diff = match received_diffs
                .into_iter()
                .max_by_key(|world_diff| world_diff.tick)
            {
                Some(world_diff) if tick_ack.0 < world_diff.tick => world_diff,
                _ => return,
            };
            tick_ack.0 = world_diff.tick;

            world.resource_scope(|world, mut entity_map: Mut<NetworkEntityMap>| {
                apply_diffs(world, &mut entity_map, world_diff, &registry);
            });
        });
    }

    fn send_client_ack_system(tick_ack: Res<TickAck>, mut client: ResMut<RenetClient>) {
        let message = rmp_serde::to_vec(&*tick_ack)
            .unwrap_or_else(|e| panic!("client ack should be serialized: {e}"));
        client.send_message(Channel::Replication as u8, message);
    }

    fn server_reset_system(mut commands: Commands) {
        commands.insert_resource(ClientAcks::default());
    }

    fn client_reset_system(mut commands: Commands) {
        commands.insert_resource(TickAck::default());
        commands.insert_resource(NetworkEntityMap::default());
    }
}

#[must_use]
fn client_diffs(
    world: &World,
    registry: &TypeRegistry,
    ignore_rules: &IgnoreRules,
    client_acks: &ClientAcks,
) -> HashMap<u64, WorldDiff> {
    let registry = registry.read();
    let mut client_diffs = HashMap::new();
    for archetype in world
        .archetypes()
        .iter()
        .filter(|archetype| archetype.id() != ArchetypeId::EMPTY)
        .filter(|archetype| archetype.id() != ArchetypeId::RESOURCE)
        .filter(|archetype| archetype.id() != ArchetypeId::INVALID)
        .filter(|archetype| !ignore_rules.ignored_archetype(archetype))
    {
        let table = world
            .storages()
            .tables
            .get(archetype.table_id())
            .expect("archetype should have a table");

        for component_id in archetype
            .components()
            .filter(|&component_id| !ignore_rules.ignored_component(archetype, component_id))
        {
            let storage_type = archetype
                .get_storage_type(component_id)
                .expect("archetype should have a storage type");

            let component_info = unsafe { world.components().get_info_unchecked(component_id) };
            let type_name = component_info.name();
            let reflect_component = component_info
                .type_id()
                .and_then(|type_id| registry.get(type_id))
                .and_then(|registration| registration.data::<ReflectComponent>())
                .expect("non-ignored component should have ReflectComponent");

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
                            &mut client_diffs,
                            *entity,
                            world,
                            client_acks,
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
                            &mut client_diffs,
                            *entity,
                            world,
                            client_acks,
                            ticks,
                            reflect_component,
                            type_name,
                        );
                    }
                }
            }
        }
    }

    client_diffs
}

fn apply_diffs(
    world: &mut World,
    entity_map: &mut NetworkEntityMap,
    world_diff: WorldDiff,
    registry: &TypeRegistry,
) {
    let read_registry = registry.read();
    for (&server_entity, components) in world_diff.entities.iter() {
        let local_entity = *entity_map
            .entry(server_entity)
            .or_insert_with(|| world.spawn().id());

        for component_diff in components.iter() {
            let type_name = component_diff.type_name();
            if let Some(reflect_component) = read_registry
                .get_with_name(type_name)
                .and_then(|registration| registration.data::<ReflectComponent>())
                .tap_none(|| error!("unable to reflect {type_name}"))
            {
                match component_diff {
                    ComponentDiff::Changed(reflect) => {
                        reflect_component.apply_or_insert(world, local_entity, &**reflect);
                    }
                    ComponentDiff::Removed(_) => reflect_component.remove(world, local_entity),
                }
            }
        }
    }
}

fn collect_if_changed(
    client_diffs: &mut HashMap<u64, WorldDiff>,
    entity: Entity,
    world: &World,
    client_acks: &ClientAcks,
    ticks: &ComponentTicks,
    reflect_component: &ReflectComponent,
    type_name: &str,
) {
    for (client_id, last_tick) in client_acks.iter() {
        let world_diff = client_diffs
            .entry(*client_id)
            .or_insert_with(|| WorldDiff::new(world.read_change_tick()));

        if ticks.is_changed(*last_tick, world.read_change_tick()) {
            let reflect = reflect_component
                .reflect(world, entity)
                .unwrap_or_else(|| panic!("unable to reflect {type_name}"))
                .clone_value();
            world_diff
                .entities
                .entry(entity)
                .or_default()
                .push(ComponentDiff::Changed(reflect));
        }
    }
}

/// Last received tick from server.
///
/// Used only on clients.
#[derive(Default, Serialize, Deserialize)]
struct TickAck(u32);

/// Last acknowledged server ticks from all clients.
///
/// Used only on server.
#[derive(Default, Deref, DerefMut)]
struct ClientAcks(HashMap<u64, u32>);

/// Maps server entities to client entities.
///
/// Used only on client.
#[derive(Default, Deref, DerefMut)]
struct NetworkEntityMap(EntityMap);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{
        game_world::GameEntity,
        network::tests::{NetworkPreset, TestNetworkPlugin},
    };

    #[test]
    fn client_acks_cleanup() {
        let mut app = App::new();
        app.init_resource::<IgnoreRules>()
            .add_plugin(TestNetworkPlugin::new(NetworkPreset::ServerAndClient {
                connected: true,
            }))
            .add_plugin(ReplicationPlugin);

        let mut client = app.world.resource_mut::<RenetClient>();
        client.disconnect();
        let client_id = client.client_id();

        let mut client_acks = app.world.resource_mut::<ClientAcks>();
        client_acks.insert(client_id, 0);

        app.update();

        let client_acks = app.world.resource::<ClientAcks>();
        assert!(!client_acks.contains_key(&client_id));
    }

    #[test]
    fn spawned_entity_replicaiton() {
        let mut app = App::new();
        app.register_type::<GameEntity>()
            .register_type::<SparseSetComponent>()
            .init_resource::<IgnoreRules>()
            .add_plugin(TestNetworkPlugin::new(NetworkPreset::ServerAndClient {
                connected: true,
            }))
            .add_plugin(ReplicationPlugin);

        app.world
            .resource_scope(|world, mut ignore_rules: Mut<IgnoreRules>| {
                ignore_rules
                    .serializable
                    .insert(world.init_component::<SparseSetComponent>());
            });

        let server_entity = app
            .world
            .spawn()
            .insert(GameEntity)
            .insert(SparseSetComponent)
            .insert(NonReflectedComponent)
            .id();

        wait_for_network_tick(&mut app);

        // Remove server entity before client replicates it (since in test client and server in the same world)
        app.world.entity_mut(server_entity).despawn();

        wait_for_network_tick(&mut app);

        let client_entity = app
            .world
            .query::<Entity>()
            .get_single(&app.world)
            .expect("server entity should be replicated to client");
        let entity_map = app.world.resource::<NetworkEntityMap>();
        let mapped_entity = entity_map
            .get(server_entity)
            .expect("server entity should be mapped on client");
        assert_eq!(
            mapped_entity, client_entity,
            "mapped entity should correspond to the replicated entity on client"
        );
        let client_entity = app.world.entity(client_entity);
        assert!(client_entity.contains::<GameEntity>());
        assert!(client_entity.contains::<SparseSetComponent>());
        assert!(!client_entity.contains::<NonReflectedComponent>());

        wait_for_network_tick(&mut app);

        let client_acks = app.world.resource::<ClientAcks>();
        let client = app.world.resource::<RenetClient>();
        assert!(client_acks.contains_key(&client.client_id()));
    }

    #[test]
    fn client_resets() {
        let mut app = App::new();
        app.add_plugin(TestNetworkPlugin::new(NetworkPreset::Client))
            .add_plugin(ReplicationPlugin);

        app.update();

        // Modify resources to test reset
        app.world.resource_mut::<TickAck>().0 += 1;
        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(Entity::from_raw(0), Entity::from_raw(0));
        app.world.remove_resource::<RenetClient>();

        app.update();

        assert_eq!(app.world.resource::<TickAck>().0, TickAck::default().0);
        assert!(app.world.resource::<NetworkEntityMap>().is_empty());
    }

    #[test]
    fn server_resets() {
        let mut app = App::new();
        app.add_plugin(TestNetworkPlugin::new(NetworkPreset::Server))
            .add_plugin(ReplicationPlugin);

        app.update();

        // Modify resources to test reset
        app.world.resource_mut::<ClientAcks>().insert(0, 0);
        app.world.remove_resource::<RenetServer>();

        app.update();

        assert_eq!(app.world.resource::<ClientAcks>().len(), 0);
    }

    fn wait_for_network_tick(app: &mut App) {
        let init_time = app.world.resource::<Time>().seconds_since_startup();
        app.update();
        while app.world.resource::<Time>().seconds_since_startup() - init_time
            < ReplicationPlugin::TIMESTEP
        {
            app.update();
        }
    }

    #[derive(Component, Default, Reflect)]
    #[component(storage = "SparseSet")]
    #[reflect(Component)]
    struct SparseSetComponent;

    #[derive(Component)]
    struct NonReflectedComponent;
}
