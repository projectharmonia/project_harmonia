pub mod editor;

use std::io::Cursor;

use bevy::{
    ecs::{
        entity::{EntityMapper, MapEntities},
        reflect::ReflectMapEntities,
    },
    prelude::*,
    reflect::serde::{ReflectSerializer, UntypedReflectDeserializer},
    utils::HashMap,
};
use bevy_replicon::{
    core::ctx::{ClientSendCtx, ServerReceiveCtx},
    prelude::*,
};
use bevy_xpbd_3d::prelude::*;
use bincode::{DefaultOptions, ErrorKind, Options};
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::actor::{Actor, ActorBundle, ReflectActorBundle, SelectedActor};
use crate::{
    component_commands::ComponentCommandsExt, core::GameState, game_world::GameWorld,
    navigation::NavigationBundle,
};
use editor::EditorPlugin;

pub struct FamilyPlugin;

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
            .add_client_event_with(
                ChannelKind::Unordered,
                serialize_family_spawn,
                deserialize_family_spawn,
            )
            .add_mapped_client_event::<FamilyDelete>(ChannelKind::Unordered)
            .add_mapped_server_event::<SelectedFamilyCreated>(ChannelKind::Unordered)
            .add_systems(
                OnEnter(GameState::Family),
                (Self::select, Self::reset_states),
            )
            .add_systems(OnExit(GameState::Family), Self::deselect)
            .add_systems(
                PreUpdate,
                (
                    Self::update_members,
                    (Self::create, Self::delete).run_if(has_authority),
                )
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<GameWorld>),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup.run_if(resource_removed::<GameWorld>()),
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
            debug!("updating family for actor `{actor_entity:?}`");

            // Remove previous.
            for mut members in &mut families {
                if let Some(index) = members.iter().position(|&entity| entity == actor_entity) {
                    members.0.swap_remove(index);
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
            info!("creating new family");
            let family_entity = commands
                .spawn(FamilyBundle::new(event.scene.name, event.scene.budget))
                .id();
            for actor in event.scene.actors {
                commands.entity(event.city_entity).with_children(|parent| {
                    parent
                        .spawn((
                            ActorFamily(family_entity),
                            ParentSync::default(),
                            Position::default(),
                            Rotation::default(),
                            NavigationBundle::default(),
                            Actor,
                            Replicated,
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
        families: Query<&mut FamilyMembers>,
    ) {
        for family_entity in delete_events.read().map(|event| event.event.0) {
            match families.get(family_entity) {
                Ok(members) => {
                    info!("deleting family `{family_entity:?}`");
                    commands.entity(family_entity).despawn();
                    for &entity in &members.0 {
                        commands.entity(entity).despawn_recursive();
                    }
                }
                Err(e) => error!("received an invalid family to despawn: {e}"),
            }
        }
    }

    pub fn select(mut commands: Commands, actors: Query<&ActorFamily, With<SelectedActor>>) {
        let family = actors.single();
        info!("selecting `{family:?}`");
        commands.entity(family.0).insert(SelectedFamily);
    }

    fn deselect(mut commands: Commands, families: Query<&ActorFamily, With<SelectedActor>>) {
        if let Ok(family) = families.get_single() {
            info!("deselecting `{family:?}`");
            commands.entity(family.0).remove::<SelectedFamily>();
        }
    }

    fn cleanup(mut commands: Commands, families: Query<Entity, With<Family>>) {
        for entity in &families {
            commands.entity(entity).despawn();
        }
    }
}

fn serialize_family_spawn(
    ctx: &mut ClientSendCtx,
    event: &FamilyCreate,
    cursor: &mut Cursor<Vec<u8>>,
) -> bincode::Result<()> {
    DefaultOptions::new().serialize_into(&mut *cursor, &event.city_entity)?;
    DefaultOptions::new().serialize_into(&mut *cursor, &event.scene.name)?;
    DefaultOptions::new().serialize_into(&mut *cursor, &event.scene.budget)?;
    DefaultOptions::new().serialize_into(&mut *cursor, &event.scene.actors.len())?;
    for actor in &event.scene.actors {
        let serializer = ReflectSerializer::new(actor.as_reflect(), ctx.registry);
        DefaultOptions::new().serialize_into(&mut *cursor, &serializer)?;
    }
    DefaultOptions::new().serialize_into(cursor, &event.select)?;

    Ok(())
}

fn deserialize_family_spawn(
    ctx: &mut ServerReceiveCtx,
    cursor: &mut Cursor<&[u8]>,
) -> bincode::Result<FamilyCreate> {
    let city_entity = DefaultOptions::new().deserialize_from(&mut *cursor)?;
    let name = DefaultOptions::new().deserialize_from(&mut *cursor)?;
    let budget = DefaultOptions::new().deserialize_from(&mut *cursor)?;
    let actors_count = DefaultOptions::new().deserialize_from(&mut *cursor)?;
    let mut actors = Vec::with_capacity(actors_count);
    for _ in 0..actors_count {
        let mut deserializer =
            bincode::Deserializer::with_reader(&mut *cursor, DefaultOptions::new());
        let reflect =
            UntypedReflectDeserializer::new(ctx.registry).deserialize(&mut deserializer)?;
        let type_info = reflect.get_represented_type_info().unwrap();
        let type_path = type_info.type_path();
        let registration = ctx
            .registry
            .get(type_info.type_id())
            .ok_or_else(|| ErrorKind::Custom(format!("{type_path} is not registered")))?;
        let reflect_actor = registration.data::<ReflectActorBundle>().ok_or_else(|| {
            ErrorKind::Custom(format!("{type_path} doesn't have reflect(ActorBundle)"))
        })?;
        let actor = reflect_actor
            .get_boxed(reflect)
            .map_err(|_| ErrorKind::Custom(format!("{type_path} is not an ActorBundle")))?;
        actors.push(actor);
    }
    let select = DefaultOptions::new().deserialize_from(cursor)?;

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
pub enum FamilyMode {
    #[default]
    Life,
    Building,
}

impl FamilyMode {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Life => "👪",
            Self::Building => "🏠",
        }
    }
}

#[derive(
    Clone, Copy, Component, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, States,
)]
pub enum BuildingMode {
    #[default]
    Objects,
    Walls,
}

impl BuildingMode {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "💺",
            Self::Walls => "🔰",
        }
    }
}

#[derive(Bundle)]
struct FamilyBundle {
    family: Family,
    budget: Budget,
    replication: Replicated,
}

impl FamilyBundle {
    fn new(name: String, budget: Budget) -> Self {
        Self {
            family: Family { name },
            budget,
            replication: Replicated,
        }
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct Family {
    pub name: String,
}

/// Indicates locally controlled family.
///
/// Inserted automatically on [`ActiveActor`] insertion.
#[derive(Component)]
pub struct SelectedFamily;

#[derive(Clone, Component, Copy, Default, Debug, Deserialize, Reflect, Serialize, Deref)]
#[reflect(Component)]
pub struct Budget(u32);

/// Contains the entities of all the actors that belong to the family.
///
/// Automatically created and updated based on [`ActorFamily`].
#[derive(Component, Default, Deref)]
pub struct FamilyMembers(Vec<Entity>);

/// Contains the family entity to which the actor belongs.
#[derive(Component, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Component, MapEntities)]
pub struct ActorFamily(pub Entity);

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
pub struct FamilyCreate {
    pub city_entity: Entity,
    pub scene: FamilyScene,
    pub select: bool,
}

impl MapEntities for FamilyCreate {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.city_entity = entity_mapper.map_entity(self.city_entity);
    }
}

#[derive(Component, Default)]
pub struct FamilyScene {
    pub name: String,
    pub budget: Budget,
    pub actors: Vec<Box<dyn ActorBundle>>,
}

impl FamilyScene {
    pub fn new(name: String) -> Self {
        Self {
            name,
            budget: Default::default(),
            actors: Default::default(),
        }
    }
}
#[derive(Clone, Copy, Deserialize, Event, Serialize)]
pub struct FamilyDelete(pub Entity);

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
