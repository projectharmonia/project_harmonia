use std::fmt::Debug;

use bevy::{ecs::event::Event, prelude::*};
use bevy_renet::renet::{RenetClient, RenetServer};
use iyes_loopless::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use tap::TapFallible;

use super::{EventChannel, NetworkEventCounter};
use crate::core::{
    game_world::GameWorld,
    network::{server::SERVER_ID, REPLICATION_CHANNEL_ID},
};

#[derive(SystemLabel)]
enum ServerEventSystems<T> {
    SendingSystem,
    #[allow(dead_code)]
    #[system_label(ignore_fields)]
    Marker(T),
}

/// An extension trait for [`App`] for creating server events.
pub(crate) trait ServerEventAppExt {
    /// Registers event `T` that will be emitted on client after adding to [`ServerSendBuffer<T>`] on server.
    fn add_server_event<T: Event + Serialize + DeserializeOwned + Debug>(&mut self) -> &mut Self;
}

impl ServerEventAppExt for App {
    fn add_server_event<T: Event + Serialize + DeserializeOwned + Debug>(&mut self) -> &mut Self {
        let mut event_counter = self
            .world
            .get_resource_or_insert_with(NetworkEventCounter::default);
        event_counter.server += 1;
        let current_channel_id = REPLICATION_CHANNEL_ID + event_counter.server;

        self.add_event::<T>()
            .init_resource::<ServerSendBuffer<T>>()
            .insert_resource(EventChannel::<T>::new(current_channel_id))
            .add_system(
                sending_system::<T>
                    .run_if_resource_exists::<RenetServer>()
                    .label(ServerEventSystems::<T>::SendingSystem),
            )
            .add_system(
                local_resending_system::<T>
                    .run_unless_resource_exists::<RenetClient>()
                    .run_if_resource_exists::<GameWorld>()
                    .after(ServerEventSystems::<T>::SendingSystem),
            )
            .add_system(receiving_system::<T>.run_if_resource_exists::<RenetClient>());

        self
    }
}

fn sending_system<T: Event + Serialize + Debug>(
    mut server: ResMut<RenetServer>,
    server_buffer: Res<ServerSendBuffer<T>>,
    channel: Res<EventChannel<T>>,
) {
    for ServerEvent { event, mode } in server_buffer.iter() {
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

/// Transforms [`EventSent<T>`] events into [`T`] events to "emulate"
/// message sending for offline mode or when server is also a player
fn local_resending_system<T: Event + Debug>(
    mut server_buffer: ResMut<ServerSendBuffer<T>>,
    mut server_events: EventWriter<T>,
) {
    for ServerEvent { event, mode } in server_buffer.drain(..) {
        match mode {
            SendMode::Broadcast => {
                debug!("converted broadcasted server event {event:?} into a local");
                server_events.send(event);
            }
            SendMode::BroadcastExcept(client_id) => {
                if client_id != SERVER_ID {
                    debug!("converted broadcasted server event {event:?} except client {client_id} into a local");
                    server_events.send(event);
                }
            }
            SendMode::Direct(client_id) => {
                if client_id == SERVER_ID {
                    debug!("converted direct server event {event:?} into a local");
                    server_events.send(event);
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
        if let Ok(event) = rmp_serde::from_slice(&message)
            .tap_ok(|event| debug!("received event {event:?} from server"))
            .tap_err(|e| error!("unable to deserialize event from server: {e}"))
        {
            server_events.send(event);
        }
    }
}

/// A container for events that will be send to clients.
///
/// Emits [`T`] event on clients.
#[derive(Deref, DerefMut)]
pub(crate) struct ServerSendBuffer<T>(Vec<ServerEvent<T>>);

impl<T> Default for ServerSendBuffer<T> {
    fn default() -> Self {
        Self(Vec::new())
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
    use bevy::ecs::event::Events;
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
            let mut server_buffer = app.world.resource_mut::<ServerSendBuffer<DummyEvent>>();
            server_buffer.push(ServerEvent {
                mode: send_mode,
                event: DummyEvent,
            });

            app.update();

            // Cleanup buffer manually when client and server are in the same world.
            app.world
                .resource_mut::<ServerSendBuffer<DummyEvent>>()
                .clear();

            app.update();

            let mut dummy_events = app.world.resource_mut::<Events<DummyEvent>>();
            assert_eq!(
                dummy_events.drain().count(),
                events_count,
                "event should be emited {events_count} times for {send_mode:?}"
            );
        }
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
            let mut server_buffer = app.world.resource_mut::<ServerSendBuffer<DummyEvent>>();
            server_buffer.push(ServerEvent {
                mode: send_mode,
                event: DummyEvent,
            });

            app.update();

            let server_buffer = app.world.resource::<ServerSendBuffer<DummyEvent>>();
            assert!(server_buffer.is_empty());

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
}
