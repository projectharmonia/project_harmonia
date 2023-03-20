use anyhow::Result;
use bevy::{
    ecs::{
        entity::{EntityMap, MapEntities, MapEntitiesError},
        reflect::ReflectMapEntities,
    },
    prelude::*,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tap::TapFallible;

use super::{
    doll::{ActiveDoll, DollBundle, PlayableDollBundle},
    game_state::GameState,
    game_world::WorldState,
    network::{
        network_event::{
            client_event::{ClientEvent, ClientEventAppExt},
            server_event::{SendMode, ServerEvent, ServerEventAppExt},
        },
        replication::{
            map_entity::ReflectMapEntity,
            replication_rules::{AppReplicationExt, Replication},
        },
        server::AuthoritySet,
    },
};

pub(super) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<FamilyMode>()
            .add_state::<BuildingMode>()
            .register_and_replicate::<FamilySync>()
            .register_and_replicate::<Budget>()
            .add_mapped_client_event::<FamilySpawn>()
            .add_mapped_client_event::<FamilyDespawn>()
            .add_mapped_server_event::<SelectedFamilySpawned>()
            .add_systems((Self::spawn_system, Self::despawn_system).in_set(AuthoritySet))
            .add_systems((
                Self::activation_system.in_schedule(OnEnter(GameState::Family)),
                Self::family_sync_system.in_set(OnUpdate(WorldState::InWorld)),
                Self::deactivation_system.in_schedule(OnExit(GameState::Family)),
                Self::cleanup_system.in_schedule(OnExit(WorldState::InWorld)),
            ));
    }
}

impl FamilyPlugin {
    fn family_sync_system(
        mut commands: Commands,
        dolls: Query<(Entity, Option<&Family>, &FamilySync), Changed<FamilySync>>,
        mut families: Query<&mut Dolls>,
    ) {
        for (entity, family, family_sync) in &dolls {
            // Remove previous.
            if let Some(family) = family {
                if let Ok(mut dolls) = families.get_mut(family.0) {
                    let index = dolls
                        .iter()
                        .position(|&doll_entity| doll_entity == entity)
                        .expect("dolls should contain referenced entity");
                    dolls.swap_remove(index);
                }
            }

            commands.entity(entity).insert(Family(family_sync.0));
            if let Ok(mut dolls) = families.get_mut(family_sync.0) {
                dolls.push(entity);
            } else {
                commands.entity(family_sync.0).insert(Dolls(vec![entity]));
            }
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: ResMut<Events<ClientEvent<FamilySpawn>>>,
        mut select_events: EventWriter<ServerEvent<SelectedFamilySpawned>>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.drain() {
            let family_entity = commands
                .spawn(FamilyBundle::new(event.scene.name, event.scene.budget))
                .id();
            for doll_bundle in event.scene.doll_bundles {
                commands.spawn(PlayableDollBundle::new(
                    doll_bundle,
                    family_entity,
                    event.city_entity,
                ));
            }
            if event.select {
                select_events.send(ServerEvent {
                    mode: SendMode::Direct(client_id),
                    event: SelectedFamilySpawned(family_entity),
                });
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<ClientEvent<FamilyDespawn>>,
        families: Query<(Entity, &mut Dolls)>,
    ) {
        for event in despawn_events.iter().map(|event| event.event) {
            if let Ok((family_entity, dolls)) = families
                .get(event.0)
                .tap_err(|e| error!("received an invalid family entity to despawn: {e}"))
            {
                commands.entity(family_entity).despawn();
                for &doll_entity in dolls.iter() {
                    commands.entity(doll_entity).despawn_recursive();
                }
            }
        }
    }

    fn activation_system(
        mut commands: Commands,
        activated_dolls: Query<&Family, Added<ActiveDoll>>,
    ) {
        commands
            .entity(activated_dolls.single().0)
            .insert(ActiveFamily);
    }

    fn deactivation_system(mut commands: Commands, active_dolls: Query<Entity, With<ActiveDoll>>) {
        commands
            .entity(active_dolls.single())
            .remove::<ActiveFamily>();
    }

    fn cleanup_system(mut commands: Commands, dolls: Query<Entity, With<Dolls>>) {
        for entity in &dolls {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub(crate) struct FamilyScene {
    pub(crate) name: Name,
    pub(crate) budget: Budget,
    pub(crate) doll_bundles: Vec<DollBundle>,
}

impl FamilyScene {
    pub(crate) fn new(name: String, doll_bundles: Vec<DollBundle>) -> Self {
        Self {
            name: Name::new(name),
            budget: Budget::default(),
            doll_bundles,
        }
    }
}

#[derive(Bundle)]
struct FamilyBundle {
    name: Name,
    budget: Budget,
    replication: Replication,
}

impl FamilyBundle {
    fn new(name: Name, budget: Budget) -> Self {
        Self {
            name,
            budget,
            replication: Replication,
        }
    }
}

#[derive(States, Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter, Default)]
pub(crate) enum FamilyMode {
    #[default]
    Life,
    Building,
}

impl FamilyMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Life => "👪",
            Self::Building => "🏠",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, States)]
pub(crate) enum BuildingMode {
    #[default]
    Objects,
    Walls,
}

impl BuildingMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "💺",
            Self::Walls => "◧",
        }
    }
}

/// Contains the family entity to which the doll belongs.
#[derive(Component)]
pub(crate) struct Family(pub(crate) Entity);

/// Contains the entities of all the dolls that belong to the family.
#[derive(Component, Default, Deref, DerefMut)]
pub(crate) struct Dolls(Vec<Entity>);

/// Contains the family entity to which the doll belongs.
///
/// Automatically updates [`Family`] and [`Dolls`] components after insertion.
#[derive(Component, Reflect)]
#[reflect(Component, MapEntities, MapEntity)]
pub(crate) struct FamilySync(pub(crate) Entity);

// We need to impl either [`FromWorld`] or [`Default`] so [`FamilySync`] can be registered as [`Reflect`].
// Same technicue is used in Bevy for [`Parent`]
impl FromWorld for FamilySync {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::from_raw(u32::MAX))
    }
}

impl MapEntities for FamilySync {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// Indicates locally controlled family.
///
/// Inserted automatically on [`ActiveDoll`] insertion.
#[derive(Component)]
pub(crate) struct ActiveFamily;

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct Budget(u32);

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct FamilySpawn {
    pub(crate) city_entity: Entity,
    pub(crate) scene: FamilyScene,
    pub(crate) select: bool,
}

impl MapEntities for FamilySpawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.city_entity = entity_map.get(self.city_entity)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct FamilyDespawn(pub(crate) Entity);

impl MapEntities for FamilyDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// An event from server which indicates spawn confirmation for the selected family.
#[derive(Deserialize, Serialize, Debug)]
pub(super) struct SelectedFamilySpawned(pub(super) Entity);

impl MapEntities for SelectedFamilySpawned {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}
