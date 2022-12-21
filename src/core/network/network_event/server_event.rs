use std::fmt::Debug;

use bevy::{
    ecs::{entity::MapEntities, event::Event},
    prelude::*,
};
use bevy_renet::renet::{RenetClient, RenetServer};
use iyes_loopless::prelude::*;
use serde::{de::DeserializeOwned, Serialize};

use super::{EventChannel, NetworkEventCounter};
use crate::core::{
    game_world::GameWorld,
    network::{
        replication::map_entity::NetworkEntityMap,
        server::{ServerFixedTimestep, SERVER_ID},
        REPLICATION_CHANNEL_ID,
    },
};

#[derive(SystemLabel)]
enum ServerEventSystem<T> {
    SendingSystem,
    #[allow(dead_code)]
    #[system_label(ignore_fields)]
    Marker(T),
}

/// An extension trait for [`App`] for creating server events.
pub(crate) trait ServerEventAppExt {
    /// Registers event `T` that will be emitted on client after sending [`ServerEvent<T>`] on server.
    fn add_server_event<T: Event + Serialize + DeserializeOwned + Debug>(&mut self) -> &mut Self;

    /// Same as [`add_server_event`], but additionally maps server entities to client after receiving.
    fn add_mapped_server_event<T: Event + Serialize + DeserializeOwned + Debug + MapEntities>(
        &mut self,
    ) -> &mut Self;

    /// Same as [`add_server_event`], but uses the specified receiving system.
    fn add_server_event_with<T: Event + Serialize + DeserializeOwned + Debug, Params>(
        &mut self,
        receiving_system: impl IntoConditionalSystem<Params>,
    ) -> &mut Self;
}

impl ServerEventAppExt for App {
    fn add_server_event<T: Event + Serialize + DeserializeOwned + Debug>(&mut self) -> &mut Self {
        self.add_server_event_with::<T, _>(receiving_system::<T>)
    }

    fn add_mapped_server_event<T: Event + Serialize + DeserializeOwned + Debug + MapEntities>(
        &mut self,
    ) -> &mut Self {
        self.add_server_event_with::<T, _>(receiving_and_mapping_system::<T>)
    }

    fn add_server_event_with<T: Event + Serialize + DeserializeOwned + Debug, Params>(
        &mut self,
        receiving_system: impl IntoConditionalSystem<Params>,
    ) -> &mut Self {
        let mut event_counter = self
            .world
            .get_resource_or_insert_with(NetworkEventCounter::default);
        event_counter.server += 1;
        let current_channel_id = REPLICATION_CHANNEL_ID + event_counter.server;

        self.add_event::<T>()
            .init_resource::<Events<ServerEvent<T>>>()
            .insert_resource(EventChannel::<T>::new(current_channel_id))
            .add_system(receiving_system.run_if_resource_exists::<RenetClient>());

        let sending_system = sending_system::<T>
            .run_if_resource_exists::<RenetServer>()
            .label(ServerEventSystem::<T>::SendingSystem);
        let local_resending_system = local_resending_system::<T>
            .run_unless_resource_exists::<RenetClient>()
            .run_if_resource_exists::<GameWorld>()
            .after(ServerEventSystem::<T>::SendingSystem);

        if cfg!(test) {
            self.add_system_to_stage(CoreStage::Update, sending_system)
                .add_system_to_stage(CoreStage::Update, local_resending_system);
        } else {
            self.add_fixed_timestep_system(ServerFixedTimestep::Tick.into(), 0, sending_system)
                .add_fixed_timestep_system(
                    ServerFixedTimestep::Tick.into(),
                    0,
                    local_resending_system,
                );
        }

        self
    }
}

fn sending_system<T: Event + Serialize + Debug>(
    mut server: ResMut<RenetServer>,
    mut server_events: EventReader<ServerEvent<T>>,
    channel: Res<EventChannel<T>>,
) {
    for ServerEvent { event, mode } in server_events.iter() {
        let message = rmp_serde::to_vec(&event).expect("unable serialize event for client(s)");

        match *mode {
            SendMode::Broadcast => {
                server.broadcast_message(channel.id, message);
                debug!("broadcasted server event {event:?}");
            }
            SendMode::BroadcastExcept(client_id) => {
                if client_id == SERVER_ID {
                    server.broadcast_message(channel.id, message);
                } else {
                    server.broadcast_message_except(client_id, channel.id, message);
                }
                debug!("broadcasted server event {event:?} except client {client_id}");
            }
            SendMode::Direct(client_id) => {
                if client_id != SERVER_ID {
                    server.send_message(client_id, channel.id, message);
                    debug!("sent direct server event {event:?} to client {client_id}");
                }
            }
        }
    }
}

/// Transforms [`ServerEvent<T>`] events into [`T`] events to "emulate"
/// message sending for offline mode or when server is also a player
fn local_resending_system<T: Event + Debug>(
    mut server_events: ResMut<Events<ServerEvent<T>>>,
    mut local_events: EventWriter<T>,
) {
    for ServerEvent { event, mode } in server_events.drain() {
        match mode {
            SendMode::Broadcast => {
                debug!("converted broadcasted server event {event:?} into a local");
                local_events.send(event);
            }
            SendMode::BroadcastExcept(client_id) => {
                if client_id != SERVER_ID {
                    debug!("converted broadcasted server event {event:?} except client {client_id} into a local");
                    local_events.send(event);
                }
            }
            SendMode::Direct(client_id) => {
                if client_id == SERVER_ID {
                    debug!("converted direct server event {event:?} into a local");
                    local_events.send(event);
                }
            }
        }
    }
}

fn receiving_system<T: Event + DeserializeOwned + Debug>(
    mut server_events: EventWriter<T>,
    mut client: ResMut<RenetClient>,
    channel: Res<EventChannel<T>>,
) {
    while let Some(message) = client.receive_message(channel.id) {
        let event = rmp_serde::from_slice(&message).expect("server should send valid events");
        debug!("received event {event:?} from server");
        server_events.send(event);
    }
}

fn receiving_and_mapping_system<T: Event + MapEntities + DeserializeOwned + Debug>(
    mut server_events: EventWriter<T>,
    mut client: ResMut<RenetClient>,
    entity_map: Res<NetworkEntityMap>,
    channel: Res<EventChannel<T>>,
) {
    while let Some(message) = client.receive_message(channel.id) {
        let mut event: T =
            rmp_serde::from_slice(&message).expect("server should send valid mapped events");
        debug!("received mapped event {event:?} from server");
        event
            .map_entities(entity_map.to_client())
            .unwrap_or_else(|e| panic!("unable to map entities for server event {event:?}: {e}"));
        server_events.send(event);
    }
}

/// An event that will be send to client(s).
#[derive(Clone, Copy, Debug)]
pub(crate) struct ServerEvent<T> {
    pub(crate) mode: SendMode,
    pub(crate) event: T,
}

/// Type of server message sending.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) enum SendMode {
    Broadcast,
    BroadcastExcept(u64),
    Direct(u64),
}

#[cfg(test)]
mod tests {
    use bevy::ecs::{
        entity::{EntityMap, MapEntitiesError},
        event::Events,
    };
    use serde::Deserialize;

    use super::*;
    use crate::core::network::network_preset::NetworkPresetPlugin;

    #[test]
    fn sending_receiving() {
        let mut app = App::new();
        app.add_server_event::<DummyEvent>()
            .add_plugin(NetworkPresetPlugin::client_and_server());

        let client_id = app.world.resource::<RenetClient>().client_id();
        for (send_mode, events_count) in [
            (SendMode::Broadcast, 1),
            (SendMode::Direct(SERVER_ID), 0),
            (SendMode::Direct(client_id), 1),
            (SendMode::BroadcastExcept(SERVER_ID), 1),
            (SendMode::BroadcastExcept(client_id), 0),
        ] {
            let mut server_events = app.world.resource_mut::<Events<ServerEvent<DummyEvent>>>();
            server_events.send(ServerEvent {
                mode: send_mode,
                event: DummyEvent,
            });

            app.update();
            app.update();

            assert_eq!(
                app.world.resource::<Events<DummyEvent>>().len(),
                events_count,
                "event should be emited {events_count} times for {send_mode:?}"
            );
        }
    }

    #[test]
    fn mapping() {
        let mut app = App::new();
        app.init_resource::<NetworkEntityMap>()
            .add_mapped_server_event::<MappedEvent>()
            .add_plugin(NetworkPresetPlugin::client_and_server());

        let client_entity = Entity::from_raw(0);
        let server_entity = Entity::from_raw(client_entity.index() + 1);
        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(server_entity, client_entity);

        let mut server_events = app.world.resource_mut::<Events<ServerEvent<MappedEvent>>>();
        server_events.send(ServerEvent {
            mode: SendMode::Broadcast,
            event: MappedEvent(server_entity),
        });

        app.update();
        app.update();

        let mapped_entities: Vec<_> = app
            .world
            .resource_mut::<Events<MappedEvent>>()
            .drain()
            .map(|event| event.0)
            .collect();
        assert_eq!(mapped_entities, [client_entity]);
    }

    #[test]
    fn local_resending() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_server_event::<DummyEvent>();

        const DUMMY_CLIENT_ID: u64 = 1;
        for (send_mode, events_count) in [
            (SendMode::Broadcast, 1),
            (SendMode::Direct(SERVER_ID), 1),
            (SendMode::Direct(DUMMY_CLIENT_ID), 0),
            (SendMode::BroadcastExcept(SERVER_ID), 0),
            (SendMode::BroadcastExcept(DUMMY_CLIENT_ID), 1),
        ] {
            let mut server_events = app.world.resource_mut::<Events<ServerEvent<DummyEvent>>>();
            server_events.send(ServerEvent {
                mode: send_mode,
                event: DummyEvent,
            });

            app.update();

            let server_events = app.world.resource::<Events<ServerEvent<DummyEvent>>>();
            assert!(server_events.is_empty());

            let mut dummy_events = app.world.resource_mut::<Events<DummyEvent>>();
            assert_eq!(
                dummy_events.drain().count(),
                events_count,
                "event should be emited {events_count} times for {send_mode:?}"
            );
        }
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct DummyEvent;

    #[derive(Deserialize, Serialize, Debug)]
    struct MappedEvent(Entity);

    impl MapEntities for MappedEvent {
        fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
            self.0 = entity_map.get(self.0)?;
            Ok(())
        }
    }
}
