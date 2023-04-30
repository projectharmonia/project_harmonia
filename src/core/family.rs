use anyhow::Result;
use bevy::{
    ecs::{
        entity::{EntityMap, MapEntities, MapEntitiesError},
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
    actor::{race::RaceComponents, ActiveActor, ActorBundle, ActorScene},
    component_commands::ComponentCommandsExt,
    game_state::GameState,
    game_world::WorldState,
};

pub(super) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<FamilyMode>()
            .add_state::<BuildingMode>()
            .replicate::<FamilySync>()
            .replicate::<Budget>()
            .add_mapped_client_event::<FamilySpawn>()
            .add_mapped_client_event::<FamilyDespawn>()
            .add_mapped_server_event::<SelectedFamilySpawned>()
            .add_systems((Self::spawn_system, Self::despawn_system).in_set(ServerSet::Authority))
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
        actors: Query<(Entity, Option<&ActorFamily>, &FamilySync), Changed<FamilySync>>,
        mut families: Query<&mut FamilyActors>,
    ) {
        let mut new_actors = HashMap::<_, Vec<_>>::new();
        for (entity, family, family_sync) in &actors {
            // Remove previous.
            if let Some(family) = family {
                if let Ok(mut actors) = families.get_mut(family.0) {
                    let index = actors
                        .iter()
                        .position(|&actor_entity| actor_entity == entity)
                        .expect("actors should contain referenced entity");
                    actors.swap_remove(index);
                }
            }

            commands.entity(entity).insert(ActorFamily(family_sync.0));
            if let Ok(mut actors) = families.get_mut(family_sync.0) {
                actors.push(entity);
            } else {
                new_actors.entry(family_sync.0).or_default().push(entity);
            }
        }

        // Apply accumulated `FamilyActors` at once in case there was no such component otherwise
        // multiple `FamilyActors` insertion with a single entity will overwrite each other.
        for (family, actors) in new_actors {
            commands.entity(family).insert(FamilyActors(actors));
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_select_events: EventWriter<ToClients<SelectedFamilySpawned>>,
        mut spawn_events: ResMut<Events<FromClient<FamilySpawn>>>,
        race_components: Res<RaceComponents>,
        registry: Res<AppTypeRegistry>,
    ) {
        let registry = registry.read();
        for FromClient { client_id, event } in spawn_events.drain() {
            let family_entity = commands
                .spawn(FamilyBundle::new(event.scene.name, event.scene.budget))
                .id();
            for actor_scene in event.scene.actor_scenes {
                let Some(registration) = registry.get_with_name(&actor_scene.race_name) else {
                    error!("type {:?} is not registered", actor_scene.race_name);
                    continue;
                };
                let Some(&bundle_id) = race_components.get(&registration.type_id()) else {
                    error!(
                        "type {:?} is not registered as a race",
                        actor_scene.race_name
                    );
                    continue;
                };

                commands
                    .spawn(ActorBundle::new(
                        actor_scene,
                        family_entity,
                        event.city_entity,
                    ))
                    .insert_default_with_id(bundle_id);
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
        families: Query<(Entity, &mut FamilyActors)>,
    ) {
        for event in despawn_events.iter().map(|event| event.event) {
            match families.get(event.0) {
                Ok((family_entity, actors)) => {
                    commands.entity(family_entity).despawn();
                    for &entity in actors.iter() {
                        commands.entity(entity).despawn_recursive();
                    }
                }
                Err(e) => error!("received an invalid family entity to despawn: {e}"),
            }
        }
    }

    fn activation_system(
        mut commands: Commands,
        activated_actors: Query<&ActorFamily, With<ActiveActor>>,
    ) {
        commands
            .entity(activated_actors.single().0)
            .insert(ActiveFamily);
    }

    fn deactivation_system(
        mut commands: Commands,
        active_actors: Query<&ActorFamily, With<ActiveActor>>,
    ) {
        commands
            .entity(active_actors.single().0)
            .remove::<ActiveFamily>();
    }

    fn cleanup_system(mut commands: Commands, actors: Query<Entity, With<FamilyActors>>) {
        for entity in &actors {
            commands.entity(entity).despawn();
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

/// Contains the family entity to which the actor belongs.
#[derive(Component)]
pub(crate) struct ActorFamily(pub(crate) Entity);

/// Contains the entities of all the actors that belong to the family.
#[derive(Component, Default, Deref, DerefMut)]
pub(crate) struct FamilyActors(Vec<Entity>);

/// Contains the family entity to which the actor belongs.
///
/// Automatically updates [`ActorFamily`] and [`FamilyActors`] components after insertion.
#[derive(Component, Reflect)]
#[reflect(Component, MapEntities)]
pub(crate) struct FamilySync(pub(crate) Entity);

// We need to impl either [`FromWorld`] or [`Default`] so [`FamilySync`] can be registered as [`Reflect`].
// Same technique is used in Bevy for [`Parent`]
impl FromWorld for FamilySync {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
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
/// Inserted automatically on [`ActiveActor`] insertion.
#[derive(Component)]
pub(crate) struct ActiveFamily;

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize, Deref)]
#[reflect(Component)]
pub(crate) struct Budget(u32);

/// Event that spawns a family.
#[derive(Debug, Serialize, Deserialize)]
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

/// Serializable family scene for gallery.
#[derive(Debug, Default, Deserialize, Serialize)]
pub(crate) struct FamilyScene {
    pub(crate) name: Name,
    budget: Budget,
    actor_scenes: Vec<ActorScene>,
}

impl FamilyScene {
    pub(crate) fn new(name: Name, actor_scenes: Vec<ActorScene>) -> Self {
        Self {
            name,
            budget: Default::default(),
            actor_scenes,
        }
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
