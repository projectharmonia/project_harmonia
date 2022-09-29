use std::marker::PhantomData;

use bevy::{ecs::event::Event, prelude::*};
use bevy_renet::renet::{RenetClient, RenetServer};
use iyes_loopless::prelude::*;
use serde::{de::DeserializeOwned, Serialize};
use tap::TapFallible;

use super::{EventChannel, NetworkEventCounter};
use crate::core::{game_world::GameWorld, network::SERVER_ID};

pub(crate) struct ServerEventPlugin<T> {
    marker: PhantomData<T>,
}

impl<T> Default for ServerEventPlugin<T> {
    fn default() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

#[derive(SystemLabel)]
enum ServerEventSystems<T> {
    SendingSystem,
    #[allow(dead_code)]
    #[system_label(ignore_fields)]
    Marker(T),
}

impl<T: Event + Serialize + DeserializeOwned> Plugin for ServerEventPlugin<T> {
    fn build(&self, app: &mut App) {
        let mut event_counter = app
            .world
            .get_resource_or_insert_with(NetworkEventCounter::default);
        let current_channel_id = event_counter.server;
        event_counter.server += 1;

        app.add_event::<T>()
            .add_event::<EventSent<T>>()
            .insert_resource(EventChannel::<T>::new(current_channel_id))
            .add_system(
                Self::sending_system
                    .run_if_resource_exists::<RenetServer>()
                    .label(ServerEventSystems::<T>::SendingSystem),
            )
            .add_system(
                Self::local_resending_system
                    .run_unless_resource_exists::<RenetClient>()
                    .run_if_resource_exists::<GameWorld>()
                    .after(ServerEventSystems::<T>::SendingSystem),
            )
            .add_system(Self::receiving_system.run_if_resource_exists::<RenetClient>());
    }
}

impl<T: Event + Serialize + DeserializeOwned> ServerEventPlugin<T> {
    fn sending_system(
        mut send_events: EventReader<EventSent<T>>,
        mut server: ResMut<RenetServer>,
        channel: Res<EventChannel<T>>,
    ) {
        for EventSent { event, mode } in send_events.iter() {
            let message = rmp_serde::to_vec(&event).expect("unable serialize event for client(s)");

            match *mode {
                SendMode::Broadcast => {
                    server.broadcast_message(channel.id, message);
                }
                SendMode::BroadcastExcept(client_id) => {
                    if client_id == SERVER_ID {
                        server.broadcast_message(channel.id, message);
                    } else {
                        server.broadcast_message_except(client_id, channel.id, message);
                    }
                }
                SendMode::Direct(client_id) => {
                    if client_id != SERVER_ID {
                        server.send_message(client_id, channel.id, message);
                    }
                }
            }
        }
    }

    /// Transforms [`EventSent<T>`] events into [`T`] events to "emulate"
    /// message sending for offline mode or when server is also a player
    fn local_resending_system(
        mut send_events: ResMut<Events<EventSent<T>>>,
        mut server_events: EventWriter<T>,
    ) {
        for EventSent { event, mode } in send_events.drain() {
            match mode {
                SendMode::Broadcast => {
                    server_events.send(event);
                }
                SendMode::BroadcastExcept(client_id) => {
                    if client_id != SERVER_ID {
                        server_events.send(event);
                    }
                }
                SendMode::Direct(client_id) => {
                    if client_id == SERVER_ID {
                        server_events.send(event);
                    }
                }
            }
        }
    }

    fn receiving_system(
        mut server_events: EventWriter<T>,
        mut client: ResMut<RenetClient>,
        channel: Res<EventChannel<T>>,
    ) {
        while let Some(message) = client.receive_message(channel.id) {
            if let Ok(event) = rmp_serde::from_slice(&message)
                .tap_err(|e| error!("unable to deserialize event from server: {e}"))
            {
                server_events.send(event);
            }
        }
    }
}

/// An event indicating that a server message has been sent.
/// This event should be used instead of sending messages directly.
/// Emited only on server.
struct EventSent<T> {
    mode: SendMode,
    event: T,
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
    use crate::core::network::tests::{NetworkPreset, TestNetworkPlugin};

    #[test]
    fn sending_receiving() {
        let mut app = App::new();
        app.add_plugin(ServerEventPlugin::<DummyEvent>::default())
            .add_plugin(TestNetworkPlugin::new(NetworkPreset::ServerAndClient {
                connected: true,
            }));

        let client_id = app.world.resource::<RenetClient>().client_id();
        for (send_mode, events_count) in [
            (SendMode::Broadcast, 1),
            (SendMode::Direct(SERVER_ID), 0),
            (SendMode::Direct(client_id), 1),
            (SendMode::BroadcastExcept(SERVER_ID), 1),
            (SendMode::BroadcastExcept(client_id), 0),
        ] {
            let mut send_events = app.world.resource_mut::<Events<EventSent<DummyEvent>>>();
            send_events.send(EventSent {
                mode: send_mode,
                event: DummyEvent,
            });

            app.update();
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
            .add_plugin(ServerEventPlugin::<DummyEvent>::default());

        const DUMMY_CLIENT_ID: u64 = 1;
        for (send_mode, events_count) in [
            (SendMode::Broadcast, 1),
            (SendMode::Direct(SERVER_ID), 1),
            (SendMode::Direct(DUMMY_CLIENT_ID), 0),
            (SendMode::BroadcastExcept(SERVER_ID), 0),
            (SendMode::BroadcastExcept(DUMMY_CLIENT_ID), 1),
        ] {
            let mut send_events = app.world.resource_mut::<Events<EventSent<DummyEvent>>>();
            send_events.send(EventSent {
                mode: send_mode,
                event: DummyEvent,
            });

            app.update();

            let mut dummy_events = app.world.resource_mut::<Events<DummyEvent>>();
            assert_eq!(
                dummy_events.drain().count(),
                events_count,
                "event should be emited {events_count} times for {send_mode:?}"
            );
        }
    }

    #[derive(Deserialize, Serialize)]
    struct DummyEvent;
}
