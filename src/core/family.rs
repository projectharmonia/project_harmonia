pub(crate) mod editor;
pub(crate) mod family_spawn;

use anyhow::Result;
use bevy::{
    ecs::{
        entity::{EntityMap, EntityMapper, MapEntities},
        reflect::ReflectMapEntities,
    },
    prelude::*,
    utils::HashMap,
};
use bevy_replicon::prelude::*;
use derive_more::Display;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    actor::{ActiveActor, ActorBundle},
    component_commands::ComponentCommandsExt,
    game_state::GameState,
    game_world::WorldName,
};
use editor::EditorPlugin;
use family_spawn::{FamilySpawn, FamilySpawnDeserializer, FamilySpawnSerializer};

pub(crate) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorPlugin)
            .add_state::<FamilyMode>()
            .add_state::<BuildingMode>()
            .replicate::<ActorFamily>()
            .replicate::<Family>()
            .replicate::<Budget>()
            .add_mapped_client_reflect_event::<FamilySpawn, FamilySpawnSerializer, FamilySpawnDeserializer>(SendPolicy::Unordered)
            .add_mapped_client_event::<FamilyDespawn>(SendPolicy::Unordered)
            .add_mapped_server_event::<SelectedFamilySpawned>(SendPolicy::Unordered)
            .add_systems(OnEnter(GameState::Family), (Self::activation_system, Self::reset_mode_system))
            .add_systems(OnExit(GameState::Family), Self::deactivation_system)
            .add_systems(PreUpdate, Self::members_update_system.after(ClientSet::Receive).run_if(resource_exists::<WorldName>()))
            .add_systems(
                Update,
                (
                    (Self::spawn_system, Self::despawn_system).run_if(has_authority()),
                    Self::cleanup_system.run_if(resource_removed::<WorldName>())
                )
            );
    }
}

impl FamilyPlugin {
    fn reset_mode_system(
        mut family_mode: ResMut<NextState<FamilyMode>>,
        mut building_mode: ResMut<NextState<BuildingMode>>,
    ) {
        family_mode.set(Default::default());
        building_mode.set(Default::default());
    }

    fn members_update_system(
        mut commands: Commands,
        actors: Query<(Entity, &ActorFamily), Changed<ActorFamily>>,
        mut families: Query<&mut FamilyMembers>,
    ) {
        let mut new_families = HashMap::<_, Vec<_>>::new();
        for (actor_entity, family) in &actors {
            // Remove previous.
            for mut members in &mut families {
                if let Some(position) = members.iter().position(|&entity| entity == actor_entity) {
                    members.0.swap_remove(position);
                    break;
                }
            }

            if let Ok(mut family) = families.get_mut(family.0) {
                family.0.push(actor_entity);
            } else {
                new_families.entry(family.0).or_default().push(actor_entity);
            }
        }

        // Apply accumulated `FamilyMembers` at once in case there was no such component otherwise
        // multiple `FamilyMembers` insertion with a single entity will overwrite each other.
        for (family_entity, members) in new_families {
            commands
                .entity(family_entity)
                .insert(FamilyMembers(members));
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_select_events: EventWriter<ToClients<SelectedFamilySpawned>>,
        mut spawn_events: ResMut<Events<FromClient<FamilySpawn>>>,
    ) {
        for FromClient { client_id, event } in spawn_events.drain() {
            let family_entity = commands
                .spawn(FamilyBundle::new(event.scene.name, event.scene.budget))
                .id();
            for race_bundle in event.scene.actors {
                commands.entity(event.city_entity).with_children(|parent| {
                    parent
                        .spawn(ActorBundle::new(family_entity))
                        .insert_reflect_bundle(race_bundle.into_reflect());
                });
            }
            if event.select {
                spawn_select_events.send(ToClients {
                    mode: SendMode::Direct(client_id),
                    event: SelectedFamilySpawned(family_entity),
                });
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<FromClient<FamilyDespawn>>,
        families: Query<(Entity, &mut FamilyMembers)>,
    ) {
        for event in despawn_events.iter().map(|event| event.event) {
            match families.get(event.0) {
                Ok((family_entity, members)) => {
                    commands.entity(family_entity).despawn();
                    for &entity in &members.0 {
                        commands.entity(entity).despawn_recursive();
                    }
                }
                Err(e) => error!("received an invalid family entity to despawn: {e}"),
            }
        }
    }

    pub(crate) fn activation_system(
        mut commands: Commands,
        actors: Query<&ActorFamily, With<ActiveActor>>,
    ) {
        commands.entity(actors.single().0).insert(ActiveFamily);
    }

    fn deactivation_system(
        mut commands: Commands,
        families: Query<&ActorFamily, With<ActiveActor>>,
    ) {
        commands
            .entity(families.single().0)
            .remove::<ActiveFamily>();
    }

    fn cleanup_system(mut commands: Commands, families: Query<Entity, With<Family>>) {
        for entity in &families {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(
    States, Component, Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter, Default,
)]
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

#[derive(
    Clone, Copy, Component, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, States,
)]
pub(crate) enum BuildingMode {
    #[default]
    Objects,
    Walls,
}

impl BuildingMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "💺",
            Self::Walls => "🔰",
        }
    }
}

#[derive(Bundle)]
struct FamilyBundle {
    name: Name,
    family: Family,
    budget: Budget,
    replication: Replication,
}

impl FamilyBundle {
    fn new(name: Name, budget: Budget) -> Self {
        Self {
            name,
            family: Family,
            budget,
            replication: Replication,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Family;

/// Indicates locally controlled family.
///
/// Inserted automatically on [`ActiveActor`] insertion.
#[derive(Component)]
pub(crate) struct ActiveFamily;

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize, Deref)]
#[reflect(Component)]
pub(crate) struct Budget(u32);

/// Contains the entities of all the actors that belong to the family.
///
/// Automatically created and updated based on [`ActorFamily`].
#[derive(Component, Default, Deref)]
pub(crate) struct FamilyMembers(Vec<Entity>);

/// Contains the family entity to which the actor belongs.
#[derive(Component, Reflect)]
#[reflect(Component, MapEntities)]
pub(crate) struct ActorFamily(pub(crate) Entity);

impl MapEntities for ActorFamily {
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
        self.0 = entity_mapper.get_or_reserve(self.0);
    }
}

// We need to impl either [`FromWorld`] or [`Default`] so [`ActorFamily`] can be registered as [`Reflect`].
// Same technique is used in Bevy for [`Parent`]
impl FromWorld for ActorFamily {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Event, Serialize)]
pub(crate) struct FamilyDespawn(pub(crate) Entity);

impl MapEventEntities for FamilyDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapError> {
        self.0 = entity_map.get(self.0).ok_or(MapError(self.0))?;
        Ok(())
    }
}

/// An event from server which indicates spawn confirmation for the selected family.
#[derive(Debug, Deserialize, Event, Serialize)]
pub(super) struct SelectedFamilySpawned(pub(super) Entity);

impl MapEventEntities for SelectedFamilySpawned {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapError> {
        self.0 = entity_map.get(self.0).ok_or(MapError(self.0))?;
        Ok(())
    }
}
