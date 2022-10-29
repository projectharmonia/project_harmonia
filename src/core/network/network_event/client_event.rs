use std::fmt::Debug;

use bevy::{
    ecs::{entity::MapEntities, event::Event},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryInternal},
};
use bevy_renet::renet::{RenetClient, RenetServer};
use iyes_loopless::prelude::*;
use rmp_serde::Deserializer;
use serde::{
    de::{DeserializeOwned, DeserializeSeed},
    Serialize,
};
use tap::TapFallible;

use super::{EventChannel, NetworkEventCounter};
use crate::core::{
    game_world::GameWorld,
    network::{
        client, replication::map_entity::NetworkEntityMap, server::SERVER_ID,
        REPLICATION_CHANNEL_ID,
    },
};

#[derive(SystemLabel)]
pub(crate) enum ClientEventSystems<T> {
    SendingSystem,
    MappingSystem,
    #[allow(dead_code)]
    #[system_label(ignore_fields)]
    Marker(T),
}

/// An extension trait for [`App`] for creating client events.
pub(crate) trait ClientEventAppExt {
    /// Registers [`ClientEvent<T>`] event that will be emitted on server after adding to [`ClientSendBuffer<T>`] on client.
    fn add_client_event<T: Event + Serialize + DeserializeOwned + Debug>(&mut self) -> &mut Self;

    /// Same as [`add_client_event`], but additionally calls [`add_client_event_mapping`].
    fn add_mapped_client_event<T: Event + Serialize + DeserializeOwned + Debug + MapEntities>(
        &mut self,
    ) -> &mut Self;

    /// Like [`add_client_event`], but uses reflection for serializaiton.
    fn add_reflect_client_event<T: Event + Debug + for<'a> ReflectEvent<'a>>(
        &mut self,
    ) -> &mut Self;

    /// Same as [`add_reflected_client_event`], but additionally calls [`add_client_event_mapping`].
    fn add_mapped_reflect_client_event<T: Event + Debug + MapEntities + for<'a> ReflectEvent<'a>>(
        &mut self,
    ) -> &mut Self;

    /// Registers [`ClientEvent<T>`] event with sending and received system.
    fn add_client_event_with<T: Event + Debug, S, R>(
        &mut self,
        sending_system: impl IntoConditionalSystem<S>,
        receiving_system: impl IntoConditionalSystem<R>,
    ) -> &mut Self;

    /// Adds a system that maps event's client entities to server before sending.
    fn add_client_event_mapping<T: Event + Debug + MapEntities>(&mut self) -> &mut Self;
}

impl ClientEventAppExt for App {
    fn add_client_event<T: Event + Serialize + DeserializeOwned + Debug>(&mut self) -> &mut Self {
        self.add_client_event_with::<T, _, _>(sending_system::<T>, receiving_system::<T>)
    }

    fn add_mapped_client_event<T: Event + Serialize + DeserializeOwned + Debug + MapEntities>(
        &mut self,
    ) -> &mut Self {
        self.add_client_event::<T>().add_client_event_mapping::<T>()
    }

    fn add_reflect_client_event<T: for<'a> ReflectEvent<'a> + Event + Debug>(
        &mut self,
    ) -> &mut Self {
        self.add_client_event_with::<T, _, _>(
            reflect_sending_system::<T>,
            reflect_receiving_system::<T>,
        )
    }

    fn add_mapped_reflect_client_event<
        T: for<'a> ReflectEvent<'a> + Event + Debug + MapEntities,
    >(
        &mut self,
    ) -> &mut Self {
        self.add_reflect_client_event::<T>()
            .add_client_event_mapping::<T>()
    }

    fn add_client_event_with<T: Event + Debug, S, R>(
        &mut self,
        sending_system: impl IntoConditionalSystem<S>,
        receiving_system: impl IntoConditionalSystem<R>,
    ) -> &mut Self {
        let mut event_counter = self
            .world
            .get_resource_or_insert_with(NetworkEventCounter::default);
        event_counter.client += 1;
        let current_channel_id = REPLICATION_CHANNEL_ID + event_counter.client;

        self.add_event::<ClientEvent<T>>()
            .init_resource::<ClientSendBuffer<T>>()
            .insert_resource(EventChannel::<T>::new(current_channel_id))
            .add_system(
                sending_system
                    .run_if(client::is_connected)
                    .label(ClientEventSystems::<T>::SendingSystem),
            )
            .add_system(
                local_resending_system::<T>
                    .run_unless_resource_exists::<RenetClient>()
                    .run_if_resource_exists::<GameWorld>(),
            )
            .add_system(receiving_system.run_if_resource_exists::<RenetServer>())
    }

    fn add_client_event_mapping<T: Event + Debug + MapEntities>(&mut self) -> &mut Self {
        self.add_system(
            mapping_system::<T>
                .run_if(client::is_connected)
                .before(ClientEventSystems::<T>::SendingSystem)
                .label(ClientEventSystems::<T>::MappingSystem),
        )
    }
}

fn mapping_system<T: Event + MapEntities + Debug>(
    mut client_buffer: ResMut<ClientSendBuffer<T>>,
    entity_map: Res<NetworkEntityMap>,
) {
    for event in client_buffer.iter_mut() {
        event
            .map_entities(entity_map.to_server())
            .unwrap_or_else(|e| panic!("unable to map entities for client event {event:?}: {e}"));
    }
}

fn sending_system<T: Event + Serialize + Debug>(
    mut client_buffer: ResMut<ClientSendBuffer<T>>,
    mut client: ResMut<RenetClient>,
    channel: Res<EventChannel<T>>,
) {
    for event in client_buffer.drain(..) {
        let message = rmp_serde::to_vec(&event).expect("unable to serialize client event");
        client.send_message(channel.id, message);
        debug!("sent client event {event:?}");
    }
}

fn reflect_sending_system<T: for<'a> ReflectEvent<'a> + Event + Debug>(
    mut client_buffer: ResMut<ClientSendBuffer<T>>,
    mut client: ResMut<RenetClient>,
    registry: Res<TypeRegistry>,
    channel: Res<EventChannel<T>>,
) {
    let registry = registry.read();
    for event in client_buffer.drain(..) {
        debug!("sent client event {event:?}");
        let serializer = T::serializer(&event, &registry);
        let message = rmp_serde::to_vec(&serializer).expect("event should be serialized");
        client.send_message(channel.id, message);
    }
}

/// Transforms [`T`] events into [`EventReceived<T>`] events to "emulate"
/// message sending for offline mode or when server is also a player
fn local_resending_system<T: Event + Debug>(
    mut client_buffer: ResMut<ClientSendBuffer<T>>,
    mut client_events: EventWriter<ClientEvent<T>>,
) {
    for event in client_buffer.drain(..) {
        debug!("converted client event {event:?} into a local");
        client_events.send(ClientEvent {
            client_id: SERVER_ID,
            event,
        })
    }
}

fn receiving_system<T: Event + Serialize + DeserializeOwned + Debug>(
    mut client_events: EventWriter<ClientEvent<T>>,
    mut server: ResMut<RenetServer>,
    channel: Res<EventChannel<T>>,
) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, channel.id) {
            if let Ok(event) = rmp_serde::from_slice(&message)
                .tap_ok(|event| debug!("received event {event:?} from client {client_id}"))
                .tap_err(|e| error!("unable to deserialize event from client {client_id}: {e}"))
            {
                client_events.send(ClientEvent { client_id, event });
            }
        }
    }
}

fn reflect_receiving_system<T: for<'a> ReflectEvent<'a> + Event + Debug>(
    mut client_events: EventWriter<ClientEvent<T>>,
    mut server: ResMut<RenetServer>,
    registry: Res<TypeRegistry>,
    channel: Res<EventChannel<T>>,
) {
    let registry = registry.read();
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, channel.id) {
            let mut deserializer = Deserializer::from_read_ref(&message);
            if let Ok(event) = T::deserializer(&registry)
                .deserialize(&mut deserializer)
                .tap_ok(|event| debug!("received event {event:?} from client {client_id}"))
                .tap_err(|e| error!("unable to deserialize event from client {client_id}: {e}"))
            {
                client_events.send(ClientEvent { client_id, event });
            }
        }
    }
}

/// A container for events that will be send to server.
///
/// Emits [`ClientEvent<T>`] on server.
#[derive(Deref, DerefMut)]
pub(crate) struct ClientSendBuffer<T>(Vec<T>);

impl<T> Default for ClientSendBuffer<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

/// An event indicating that a message from client was received.
/// Emited only on server.
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) struct ClientEvent<T> {
    pub(crate) client_id: u64,
    pub(crate) event: T,
}

pub(crate) trait ReflectEvent<'a> {
    type Serializer: Serialize;
    type Deserializer: DeserializeSeed<'a, Value = Self>;

    fn serializer(event: &'a Self, registry: &'a TypeRegistryInternal) -> Self::Serializer;
    fn deserializer(registry: &'a TypeRegistryInternal) -> Self::Deserializer;
}

#[cfg(test)]
mod tests {
    use bevy::ecs::{
        entity::{EntityMap, MapEntitiesError},
        event::Events,
    };
    use serde::Deserialize;

    use super::*;
    use crate::core::{
        family::family_scene::{FamilyScene, FamilySpawn},
        network::network_preset::NetworkPresetPlugin,
    };

    #[test]
    fn sending_receiving() {
        let mut app = App::new();
        app.init_resource::<NetworkEntityMap>()
            .add_mapped_client_event::<DummyEvent>()
            .add_plugin(NetworkPresetPlugin::client_and_server());

        let client_entity = Entity::from_raw(0);
        let server_entity = Entity::from_raw(client_entity.id() + 1);
        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(server_entity, client_entity);

        let mut dummy_buffer = app.world.resource_mut::<ClientSendBuffer<DummyEvent>>();
        dummy_buffer.push(DummyEvent(client_entity));

        app.update();

        let dummy_buffer = app.world.resource::<ClientSendBuffer<DummyEvent>>();
        assert!(dummy_buffer.is_empty());

        app.update();

        let mut client_events = app.world.resource_mut::<Events<ClientEvent<DummyEvent>>>();
        itertools::assert_equal(
            client_events.drain().map(|event| event.event.0),
            [server_entity],
        );
    }

    #[test]
    fn reflect_sending_receiving() {
        let mut app = App::new();
        app.init_resource::<NetworkEntityMap>()
            .add_mapped_reflect_client_event::<FamilySpawn>()
            .add_plugin(NetworkPresetPlugin::client_and_server());

        let client_entity = Entity::from_raw(0);
        let server_entity = Entity::from_raw(client_entity.id() + 1);
        app.world
            .resource_mut::<NetworkEntityMap>()
            .insert(server_entity, client_entity);

        let mut spawn_buffer = app.world.resource_mut::<ClientSendBuffer<FamilySpawn>>();
        spawn_buffer.push(FamilySpawn {
            city_entity: client_entity,
            scene: FamilyScene::default(),
        });

        app.update();

        let spawn_buffer = app.world.resource::<ClientSendBuffer<FamilySpawn>>();
        assert!(spawn_buffer.is_empty());

        app.update();

        let mut client_events = app.world.resource_mut::<Events<ClientEvent<FamilySpawn>>>();
        itertools::assert_equal(
            client_events.drain().map(|event| event.event.city_entity),
            [server_entity],
        );
    }

    #[test]
    fn local_resending() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_mapped_client_event::<DummyEvent>();

        let mut dummy_buffer = app.world.resource_mut::<ClientSendBuffer<DummyEvent>>();
        dummy_buffer.push(DummyEvent(Entity::from_raw(0)));

        app.update();

        let dummy_buffer = app.world.resource::<ClientSendBuffer<DummyEvent>>();
        assert!(dummy_buffer.is_empty());

        let mut client_events = app.world.resource_mut::<Events<ClientEvent<DummyEvent>>>();
        assert_eq!(client_events.drain().count(), 1);
    }

    #[derive(Deserialize, Serialize, Debug)]
    struct DummyEvent(Entity);

    impl MapEntities for DummyEvent {
        fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
            self.0 = entity_map.get(self.0)?;
            Ok(())
        }
    }
}
