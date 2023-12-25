pub(crate) mod editor;

use std::io::Cursor;

use anyhow::{anyhow, Context, Result};
use bevy::{
    ecs::{
        entity::{EntityMapper, MapEntities},
        reflect::ReflectMapEntities,
    },
    prelude::*,
    reflect::{
        serde::{ReflectSerializer, UntypedReflectDeserializer},
        TypeRegistry,
    },
    utils::HashMap,
};
use bevy_replicon::prelude::*;
use bincode::{DefaultOptions, Options};
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::{
    actor::{ActiveActor, Actor, ActorBundle, ReflectActorBundle},
    component_commands::ComponentCommandsExt,
    game_state::GameState,
    game_world::WorldName,
};
use editor::EditorPlugin;

pub(crate) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorPlugin)
            .add_state::<FamilyMode>()
            .add_state::<BuildingMode>()
            .register_type::<ActorFamily>()
            .register_type::<Family>()
            .register_type::<Budget>()
            .replicate::<ActorFamily>()
            .replicate::<Family>()
            .replicate::<Budget>()
            .add_client_event_with::<FamilySpawn, _, _>(
                EventType::Unordered,
                Self::sending_spawn_system,
                Self::receiving_spawn_system,
            )
            .add_mapped_client_event::<FamilyDespawn>(EventType::Unordered)
            .add_mapped_server_event::<SelectedFamilySpawned>(EventType::Unordered)
            .add_systems(
                OnEnter(GameState::Family),
                (Self::activation_system, Self::reset_mode_system),
            )
            .add_systems(OnExit(GameState::Family), Self::deactivation_system)
            .add_systems(
                PreUpdate,
                Self::members_update_system
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                Update,
                (Self::spawn_system, Self::despawn_system).run_if(has_authority()),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup_system.run_if(resource_removed::<WorldName>()),
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
            for actor in event.scene.actors {
                commands.entity(event.city_entity).with_children(|parent| {
                    parent
                        .spawn((
                            ActorFamily(family_entity),
                            ParentSync::default(),
                            Transform::default(),
                            Actor,
                            Replication,
                        ))
                        .insert_reflect_bundle(actor.into_reflect());
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
        for event in despawn_events.read().map(|event| event.event) {
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
        if let Ok(family) = families.get_single() {
            commands.entity(family.0).remove::<ActiveFamily>();
        }
    }

    fn cleanup_system(mut commands: Commands, families: Query<Entity, With<Family>>) {
        for entity in &families {
            commands.entity(entity).despawn();
        }
    }

    fn sending_spawn_system(
        mut spawn_events: EventReader<FamilySpawn>,
        mut client: ResMut<RenetClient>,
        channel: Res<ClientEventChannel<FamilySpawn>>,
        registry: Res<AppTypeRegistry>,
    ) {
        let registry = registry.read();
        for event in spawn_events.read() {
            let message = serialize_family_spawn(event, &registry)
                .expect("client event should be serializable");

            client.send_message(*channel, message);
        }
    }

    fn receiving_spawn_system(
        mut spawn_events: EventWriter<FromClient<FamilySpawn>>,
        mut server: ResMut<RenetServer>,
        channel: Res<ServerEventChannel<FamilySpawn>>,
        registry: Res<AppTypeRegistry>,
        entity_map: Res<ServerEntityMap>,
    ) {
        let registry = registry.read();
        for client_id in server.clients_id() {
            while let Some(message) = server.receive_message(client_id, *channel) {
                match deserialize_family_spawn(&message, &registry) {
                    Ok(mut event) => {
                        event.map_entities(&mut EventMapper(entity_map.to_server()));
                        spawn_events.send(FromClient { client_id, event });
                    }
                    Err(e) => {
                        error!("unable to deserialize event from client {client_id}: {e}")
                    }
                }
            }
        }
    }
}

fn serialize_family_spawn(
    event: &FamilySpawn,
    registry: &TypeRegistry,
) -> bincode::Result<Vec<u8>> {
    let mut message = Vec::new();
    DefaultOptions::new().serialize_into(&mut message, &event.city_entity)?;
    DefaultOptions::new().serialize_into(&mut message, &event.scene.name)?;
    DefaultOptions::new().serialize_into(&mut message, &event.scene.budget)?;
    DefaultOptions::new().serialize_into(&mut message, &event.scene.actors.len())?;
    for actor in &event.scene.actors {
        let serializer = ReflectSerializer::new(actor.as_reflect(), registry);
        DefaultOptions::new().serialize_into(&mut message, &serializer)?;
    }
    DefaultOptions::new().serialize_into(&mut message, &event.select)?;

    Ok(message)
}

fn deserialize_family_spawn(message: &[u8], registry: &TypeRegistry) -> Result<FamilySpawn> {
    let mut cursor = Cursor::new(message);
    let city_entity = DefaultOptions::new().deserialize_from(&mut cursor)?;
    let name = DefaultOptions::new().deserialize_from(&mut cursor)?;
    let budget = DefaultOptions::new().deserialize_from(&mut cursor)?;
    let actors_count = DefaultOptions::new().deserialize_from(&mut cursor)?;
    let mut actors = Vec::with_capacity(actors_count);
    for _ in 0..actors_count {
        let mut deserializer =
            bincode::Deserializer::with_reader(&mut cursor, DefaultOptions::new());
        let reflect = UntypedReflectDeserializer::new(registry).deserialize(&mut deserializer)?;
        let type_info = reflect.get_represented_type_info().unwrap();
        let type_path = type_info.type_path();
        let registration = registry
            .get(type_info.type_id())
            .with_context(|| format!("{type_path} is not registered"))?;
        let reflect_actor = registration
            .data::<ReflectActorBundle>()
            .with_context(|| format!("{type_path} doesn't have reflect(ActorBundle)"))?;
        let actor = reflect_actor
            .get_boxed(reflect)
            .map_err(|_| anyhow!("{type_path} is not an ActorBundle"))?;
        actors.push(actor);
    }
    let select = DefaultOptions::new().deserialize_from(&mut cursor)?;

    Ok(FamilySpawn {
        city_entity,
        scene: FamilyScene {
            name,
            budget,
            actors,
        },
        select,
    })
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

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(crate) struct Family;

/// Indicates locally controlled family.
///
/// Inserted automatically on [`ActiveActor`] insertion.
#[derive(Component)]
pub(crate) struct ActiveFamily;

#[derive(Clone, Component, Copy, Default, Deserialize, Reflect, Serialize, Deref)]
#[reflect(Component)]
pub(crate) struct Budget(u32);

/// Contains the entities of all the actors that belong to the family.
///
/// Automatically created and updated based on [`ActorFamily`].
#[derive(Component, Default, Deref)]
pub(crate) struct FamilyMembers(Vec<Entity>);

/// Contains the family entity to which the actor belongs.
#[derive(Component, Reflect, Serialize, Deserialize)]
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

#[derive(Event)]
pub(crate) struct FamilySpawn {
    pub(crate) city_entity: Entity,
    pub(crate) scene: FamilyScene,
    pub(crate) select: bool,
}

impl MapNetworkEntities for FamilySpawn {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.city_entity = mapper.map(self.city_entity);
    }
}

#[derive(Component, Default)]
pub(crate) struct FamilyScene {
    pub(crate) name: Name,
    pub(crate) budget: Budget,
    pub(crate) actors: Vec<Box<dyn ActorBundle>>,
}

impl FamilyScene {
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            budget: Default::default(),
            actors: Default::default(),
        }
    }
}
#[derive(Clone, Copy, Deserialize, Event, Serialize)]
pub(crate) struct FamilyDespawn(pub(crate) Entity);

impl MapNetworkEntities for FamilyDespawn {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.0 = mapper.map(self.0);
    }
}

/// An event from server which indicates spawn confirmation for the selected family.
#[derive(Deserialize, Event, Serialize)]
pub(super) struct SelectedFamilySpawned(pub(super) Entity);

impl MapNetworkEntities for SelectedFamilySpawned {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.0 = mapper.map(self.0);
    }
}
