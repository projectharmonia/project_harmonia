use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_renet::renet::RenetServer;
use derive_more::Display;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

use super::{
    family::FamilySync,
    game_state::GameState,
    game_world::{parent_sync::ParentSync, GameEntity, GameWorld},
    network::network_event::client_event::{ClientEvent, ClientEventAppExt},
    task::QueuedTasks,
};

pub(super) struct DollPlugin;

impl Plugin for DollPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FirstName>()
            .register_type::<LastName>()
            .register_type::<DollPlayers>()
            .add_mapped_client_event::<DollSelect>()
            .add_client_event::<DollDeselect>()
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::name_update_system.run_if_resource_exists::<GameWorld>())
            .add_enter_system(
                GameState::Family,
                Self::activation_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_system(
                Self::activation_confirmation_system.run_if_resource_exists::<RenetServer>(),
            )
            .add_exit_system(GameState::Family, Self::deactivation_system)
            .add_system(
                Self::deactivation_confirmation_system.run_if_resource_exists::<RenetServer>(),
            );
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

    fn activation_system(
        mut select_events: EventWriter<DollSelect>,
        new_active_dolls: Query<Entity, Added<ActiveDoll>>,
    ) {
        select_events.send(DollSelect(new_active_dolls.single()));
    }

    fn activation_confirmation_system(
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

    fn deactivation_system(
        mut commands: Commands,
        mut deselect_events: EventWriter<DollDeselect>,
        active_dolls: Query<Entity, With<ActiveDoll>>,
    ) {
        commands
            .entity(active_dolls.single())
            .remove::<ActiveDoll>();
        deselect_events.send(DollDeselect);
    }

    fn deactivation_confirmation_system(
        mut commands: Commands,
        mut deselect_events: EventReader<ClientEvent<DollDeselect>>,
        mut doll_players: Query<(Entity, &mut DollPlayers)>,
    ) {
        for client_id in deselect_events.iter().map(|event| event.client_id) {
            for (entity, mut doll_players) in &mut doll_players {
                if let Some(index) = doll_players.iter().position(|&id| id == client_id) {
                    if doll_players.len() == 1 {
                        commands.entity(entity).remove::<DollPlayers>();
                    } else {
                        doll_players.swap_remove(index);
                    }
                    break;
                }
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

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct DollDeselect;

#[derive(Bundle)]
pub(crate) struct DollBundle {
    first_name: FirstName,
    last_name: LastName,
    family_sync: FamilySync,
    parent_sync: ParentSync,
    transform: Transform,
    queued_tasks: QueuedTasks,
    game_entity: GameEntity,
}

impl DollBundle {
    pub(crate) fn new(
        first_name: FirstName,
        last_name: LastName,
        family_entity: Entity,
        city_entity: Entity,
    ) -> Self {
        Self {
            first_name,
            last_name,
            family_sync: FamilySync(family_entity),
            parent_sync: ParentSync(city_entity),
            transform: Default::default(),
            queued_tasks: Default::default(),
            game_entity: GameEntity,
        }
    }
}
