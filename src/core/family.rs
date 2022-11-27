use anyhow::Result;
use bevy::{
    ecs::{
        entity::{EntityMap, MapEntities, MapEntitiesError},
        reflect::ReflectMapEntities,
    },
    prelude::*,
};
use bevy_renet::renet::RenetServer;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use tap::TapFallible;

use super::{
    city::ActiveCity,
    doll::{ActiveDoll, DollBundle, DollScene, DollSelect},
    game_state::GameState,
    game_world::GameWorld,
    network::{
        network_event::client_event::{ClientEvent, ClientEventAppExt, ClientSendBuffer},
        replication::map_entity::ReflectMapEntity,
    },
    task::QueuedTasks,
};

#[derive(SystemLabel)]
pub(crate) enum FamilySystems {
    SaveSystem,
}

pub(super) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FamilySync>()
            .register_type::<Budget>()
            .add_mapped_client_event::<FamilySpawn>()
            .add_mapped_client_event::<FamilyDespawn>()
            .add_event::<FamilySelect>()
            .add_system(Self::family_sync_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::spawn_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::selection_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::select_confirmation_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::despawn_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::state_enter_system.run_in_state(GameState::World))
            .add_system(Self::state_enter_system.run_in_state(GameState::FamilyEditor))
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>());
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
        mut select_events: EventWriter<ClientEvent<DollSelect>>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.drain() {
            let family_entity = commands.spawn((event.scene.name, event.scene.budget)).id();
            commands.entity(event.city_entity).with_children(|parent| {
                for (index, doll_scene) in event.scene.dolls.into_iter().enumerate() {
                    let doll_entity = parent
                        .spawn((
                            DollBundle {
                                first_name: doll_scene.first_name,
                                last_name: doll_scene.last_name,
                                ..Default::default()
                            },
                            QueuedTasks::default(),
                            FamilySync(family_entity),
                        ))
                        .id();
                    if index == 0 && event.select {
                        select_events.send(ClientEvent {
                            client_id,
                            event: DollSelect(doll_entity),
                        })
                    }
                }
            });
        }
    }

    /// Transforms [`FamilySelect`] events into [`DollSelect`] events with the first doll.
    fn selection_system(
        mut doll_select_buffer: ResMut<ClientSendBuffer<DollSelect>>,
        mut family_select_events: EventReader<FamilySelect>,
        families: Query<&Dolls>,
    ) {
        for event in family_select_events.iter() {
            let family = families
                .get(event.0)
                .expect("event entity should be a family");
            let doll_entity = *family
                .first()
                .expect("spawned family should always contain at least one doll");
            doll_select_buffer.push(DollSelect(doll_entity));
        }
    }

    fn select_confirmation_system(
        mut commands: Commands,
        active_dolls: Query<&Family, Added<ActiveDoll>>,
        active_families: Query<Entity, With<ActiveFamily>>,
    ) {
        for family in &active_dolls {
            if let Ok(previous_entity) = active_families.get_single() {
                commands.entity(previous_entity).remove::<ActiveFamily>();
            }
            commands.entity(family.0).insert(ActiveFamily);
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
                for &entity in dolls.iter() {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }

    fn state_enter_system(
        mut commands: Commands,
        parents: Query<&Parent>,
        active_families: Query<&Dolls, Added<ActiveFamily>>,
    ) {
        if let Ok(dolls) = active_families.get_single() {
            let doll_entity = *dolls
                .first()
                .expect("family should contain at least one doll");
            let parent = parents
                .get(doll_entity)
                .expect("doll should have a city as a parent");
            commands.entity(parent.get()).insert(ActiveCity);
            commands.insert_resource(NextState(GameState::Family));
        }
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
    pub(crate) dolls: Vec<DollScene>,
}

impl FamilyScene {
    pub(crate) fn new(name: String, dolls: Vec<DollScene>) -> Self {
        Self {
            name: Name::new(name),
            budget: Budget::default(),
            dolls,
        }
    }
}

#[derive(Component)]
pub(crate) struct Family(pub(crate) Entity);

#[derive(Component, Default, Deref, DerefMut)]
pub(crate) struct Dolls(Vec<Entity>);

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

/// Selects a family entity to play using its first doll.
///
/// Its a local event that translates into a [`DollSelect`]
/// event with the first doll from selected family.
pub(crate) struct FamilySelect(pub(crate) Entity);

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct FamilyDespawn(pub(crate) Entity);

impl MapEntities for FamilyDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}
