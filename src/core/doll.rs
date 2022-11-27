use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_renet::renet::{RenetClient, RenetServer};
use derive_more::Display;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use super::{
    game_world::GameWorld,
    network::{
        network_event::client_event::{ClientEvent, ClientEventAppExt},
        server::SERVER_ID,
    },
};

pub(super) struct DollPlugin;

impl Plugin for DollPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FirstName>()
            .register_type::<LastName>()
            .register_type::<DollPlayers>()
            .add_mapped_client_event::<DollSelect>()
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::name_update_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::selection_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::select_confirmation_system.run_if_resource_exists::<GameWorld>());
    }
}

impl DollPlugin {
    fn init_system(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        new_dolls: Query<Entity, Added<FirstName>>,
    ) {
        for entity in &new_dolls {
            commands.entity(entity).insert((
                VisibilityBundle::default(),
                GlobalTransform::default(),
                meshes.add(Mesh::from(shape::Capsule::default())),
                materials.add(Color::rgb(0.3, 0.3, 0.3).into()),
            ));
        }
    }

    fn name_update_system(
        mut commands: Commands,
        mut changed_names: Query<
            (Entity, &FirstName, &LastName),
            Or<(Changed<FirstName>, Changed<LastName>)>,
        >,
    ) {
        for (entity, first_name, last_name) in &mut changed_names {
            commands
                .entity(entity)
                .insert(Name::new(format!("{first_name} {last_name}")));
        }
    }

    fn selection_system(
        mut commands: Commands,
        mut select_events: EventReader<ClientEvent<DollSelect>>,
        mut doll_players: Query<&mut DollPlayers>,
    ) {
        for ClientEvent { client_id, event } in select_events.iter().copied() {
            // Remove previous.
            for mut doll_players in &mut doll_players {
                if let Some(index) = doll_players.iter().position(|&id| id == client_id) {
                    if doll_players.len() == 1 {
                        commands.entity(event.0).remove::<DollPlayers>();
                    } else {
                        doll_players.swap_remove(index);
                    }
                    break;
                }
            }

            if let Ok(mut doll_players) = doll_players.get_mut(event.0) {
                doll_players.push(client_id);
            } else {
                commands
                    .entity(event.0)
                    .insert(DollPlayers(smallvec![client_id]));
            }
        }
    }

    fn select_confirmation_system(
        mut commands: Commands,
        client: Option<ResMut<RenetClient>>,
        doll_players: Query<(Entity, &DollPlayers), Changed<DollPlayers>>,
        active_dolls: Query<Entity, With<ActiveDoll>>,
    ) {
        let client_id = client.map(|client| client.client_id()).unwrap_or(SERVER_ID);
        for (doll_entity, doll_players) in &doll_players {
            if doll_players.contains(&client_id) {
                if let Ok(previous_entity) = active_dolls.get_single() {
                    commands.entity(previous_entity).remove::<ActiveDoll>();
                }
                commands.entity(doll_entity).insert(ActiveDoll);
                break;
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub(crate) struct DollScene {
    pub(crate) first_name: FirstName,
    pub(crate) last_name: LastName,
}

#[derive(Clone, Component, Debug, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct FirstName(pub(crate) String);

#[derive(Clone, Component, Debug, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct LastName(pub(crate) String);

/// Contains list of player IDs who controls this doll.
#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(crate) struct DollPlayers(SmallVec<[u64; 2]>);

/// Indicates locally controlled doll.
#[derive(Component)]
pub(crate) struct ActiveDoll;

/// Selects a doll entity to play.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct DollSelect(pub(crate) Entity);

impl MapEntities for DollSelect {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

#[derive(Bundle, Default)]
pub(crate) struct DollBundle {
    pub(crate) first_name: FirstName,
    pub(crate) last_name: LastName,
    pub(crate) transform: Transform,
}
