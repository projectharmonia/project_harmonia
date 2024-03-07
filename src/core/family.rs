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
    actor::{Actor, ActorBundle, ReflectActorBundle, SelectedActor},
    component_commands::ComponentCommandsExt,
    game_state::GameState,
    game_world::WorldName,
};
use editor::EditorPlugin;

pub(crate) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(EditorPlugin)
            .init_state::<FamilyMode>()
            .init_state::<BuildingMode>()
            .register_type::<ActorFamily>()
            .register_type::<Family>()
            .register_type::<Budget>()
            .replicate::<ActorFamily>()
            .replicate::<Family>()
            .replicate::<Budget>()
            .add_client_event_with::<FamilyCreate, _, _>(
                EventType::Unordered,
                Self::send_spawns,
                Self::receive_spawns,
            )
            .add_mapped_client_event::<FamilyDelete>(EventType::Unordered)
            .add_mapped_server_event::<SelectedFamilyCreated>(EventType::Unordered)
            .add_systems(
                OnEnter(GameState::Family),
                (Self::select, Self::reset_states),
            )
            .add_systems(OnExit(GameState::Family), Self::remove_selection)
            .add_systems(
                PreUpdate,
                (
                    Self::update_members,
                    (Self::create, Self::delete).run_if(has_authority),
                )
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup.run_if(resource_removed::<WorldName>()),
            );
    }
}

impl FamilyPlugin {
    fn reset_states(
        mut family_mode: ResMut<NextState<FamilyMode>>,
        mut building_mode: ResMut<NextState<BuildingMode>>,
    ) {
        family_mode.set(Default::default());
        building_mode.set(Default::default());
    }

    fn update_members(
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

    fn create(
        mut commands: Commands,
        mut created_events: EventWriter<ToClients<SelectedFamilyCreated>>,
        mut create_events: ResMut<Events<FromClient<FamilyCreate>>>,
    ) {
        for FromClient { client_id, event } in create_events.drain() {
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
                created_events.send(ToClients {
                    mode: SendMode::Direct(client_id),
                    event: SelectedFamilyCreated(family_entity),
                });
            }
        }
    }

    fn delete(
        mut commands: Commands,
        mut delete_events: EventReader<FromClient<FamilyDelete>>,
        families: Query<(Entity, &mut FamilyMembers)>,
    ) {
        for entity in delete_events.read().map(|event| event.event.0) {
            match families.get(entity) {
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

    pub(crate) fn select(mut commands: Commands, actors: Query<&ActorFamily, With<SelectedActor>>) {
        commands.entity(actors.single().0).insert(SelectedFamily);
    }

    fn remove_selection(
        mut commands: Commands,
        families: Query<&ActorFamily, With<SelectedActor>>,
    ) {
        if let Ok(family) = families.get_single() {
            commands.entity(family.0).remove::<SelectedFamily>();
        }
    }

    fn cleanup(mut commands: Commands, families: Query<Entity, With<Family>>) {
        for entity in &families {
            commands.entity(entity).despawn();
        }
    }

    fn send_spawns(
        mut spawn_events: EventReader<FamilyCreate>,
        mut client: ResMut<RenetClient>,
        channel: Res<ClientEventChannel<FamilyCreate>>,
        registry: Res<AppTypeRegistry>,
    ) {
        let registry = registry.read();
        for event in spawn_events.read() {
            let message = serialize_family_spawn(event, &registry)
                .expect("client event should be serializable");

            client.send_message(*channel, message);
        }
    }

    fn receive_spawns(
        mut spawn_events: EventWriter<FromClient<FamilyCreate>>,
        mut server: ResMut<RenetServer>,
        channel: Res<ServerEventChannel<FamilyCreate>>,
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
    event: &FamilyCreate,
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

fn deserialize_family_spawn(message: &[u8], registry: &TypeRegistry) -> Result<FamilyCreate> {
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

    Ok(FamilyCreate {
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
pub(crate) struct SelectedFamily;

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
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
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
pub(crate) struct FamilyCreate {
    pub(crate) city_entity: Entity,
    pub(crate) scene: FamilyScene,
    pub(crate) select: bool,
}

impl MapEntities for FamilyCreate {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.city_entity = entity_mapper.map_entity(self.city_entity);
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
pub(crate) struct FamilyDelete(pub(crate) Entity);

impl MapEntities for FamilyDelete {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

/// An event from server which indicates spawn confirmation for the selected family.
#[derive(Deserialize, Event, Serialize)]
pub(super) struct SelectedFamilyCreated(pub(super) Entity);

impl MapEntities for SelectedFamilyCreated {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}
