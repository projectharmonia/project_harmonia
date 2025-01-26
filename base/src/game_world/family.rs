pub mod building;
pub mod editor;

use std::io::Cursor;

use bevy::{
    ecs::{
        entity::{EntityMapper, MapEntities},
        reflect::ReflectCommandExt,
    },
    prelude::*,
    reflect::serde::{ReflectDeserializer, ReflectSerializer},
};
use bevy_replicon::{
    core::event_registry::ctx::{ClientSendCtx, ServerReceiveCtx},
    prelude::*,
};
use bincode::{DefaultOptions, ErrorKind, Options};
use serde::{de::DeserializeSeed, Deserialize, Serialize};
use strum::EnumIter;

use super::{
    actor::{Actor, SelectedActor},
    WorldState,
};
use crate::core::GameState;
use building::BuildingPlugin;
use editor::{EditorPlugin, FamilyScene, ReflectActorBundle};

pub struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((EditorPlugin, BuildingPlugin))
            .add_sub_state::<FamilyMode>()
            .enable_state_scoped_entities::<FamilyMode>()
            .register_type::<Family>()
            .register_type::<Budget>()
            .replicate::<Budget>()
            .replicate_group::<(Family, Name)>()
            .add_client_event_with(
                ChannelKind::Unordered,
                serialize_family_spawn,
                deserialize_family_spawn,
            )
            .add_mapped_client_event::<FamilyDelete>(ChannelKind::Unordered)
            .add_mapped_server_event::<SelectedFamilyCreated>(ChannelKind::Unordered)
            .add_observer(Self::record_new_members)
            .add_observer(Self::update_members)
            .add_systems(OnEnter(WorldState::Family), Self::select)
            .add_systems(
                OnExit(WorldState::Family),
                Self::deselect.never_param_warn(),
            )
            .add_systems(
                PreUpdate,
                (Self::create, Self::delete)
                    .run_if(server_or_singleplayer)
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl FamilyPlugin {
    fn record_new_members(
        trigger: Trigger<OnAdd, Actor>,
        mut commands: Commands,
        actors: Query<&Actor>,
    ) {
        let actor = actors.get(trigger.entity()).unwrap();
        commands.trigger_targets(FamilyMemberAdded(trigger.entity()), actor.family_entity);
    }

    fn update_members(
        trigger: Trigger<FamilyMemberAdded>,
        mut families: Query<&mut FamilyMembers>,
    ) {
        let mut members = families.get_mut(trigger.entity()).unwrap();
        members.push(**trigger)
    }

    fn create(
        mut commands: Commands,
        mut created_events: EventWriter<ToClients<SelectedFamilyCreated>>,
        mut create_events: ResMut<Events<FromClient<FamilyCreate>>>,
    ) {
        for FromClient { client_id, event } in create_events.drain() {
            info!("creating new family");
            let family_entity = commands.spawn((Family, Name::new(event.scene.name))).id();
            for actor in event.scene.actors {
                commands.entity(event.city_entity).with_children(|parent| {
                    parent
                        .spawn(Actor { family_entity })
                        .insert_reflect(actor.into_partial_reflect());
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
                    info!("deleting family `{family_entity}`");
                    commands.entity(family_entity).despawn();
                    for &entity in &members.0 {
                        commands.entity(entity).despawn_recursive();
                    }
                }
                Err(e) => error!("received an invalid family to despawn: {e}"),
            }
        }
    }

    pub fn select(mut commands: Commands, selected_actor: Single<&Actor, With<SelectedActor>>) {
        info!("selecting `{}`", selected_actor.family_entity);
        commands
            .entity(selected_actor.family_entity)
            .insert(SelectedFamily);
    }

    fn deselect(mut commands: Commands, selected_actor: Single<&Actor, With<SelectedActor>>) {
        info!("deselecting `{}`", selected_actor.family_entity);
        commands
            .entity(selected_actor.family_entity)
            .remove::<SelectedFamily>();
    }
}

fn serialize_family_spawn(
    ctx: &mut ClientSendCtx,
    event: &FamilyCreate,
    cursor: &mut Cursor<Vec<u8>>,
) -> bincode::Result<()> {
    DefaultOptions::new().serialize_into(&mut *cursor, &event.city_entity)?;
    DefaultOptions::new().serialize_into(&mut *cursor, &event.scene.name)?;
    DefaultOptions::new().serialize_into(&mut *cursor, &event.scene.actors.len())?;
    for actor in &event.scene.actors {
        let serializer = ReflectSerializer::new(actor.as_partial_reflect(), ctx.registry);
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
    let actors_count = DefaultOptions::new().deserialize_from(&mut *cursor)?;
    let mut actors = Vec::with_capacity(actors_count);
    for _ in 0..actors_count {
        let mut deserializer =
            bincode::Deserializer::with_reader(&mut *cursor, DefaultOptions::new());
        let partial_reflect =
            ReflectDeserializer::new(ctx.registry).deserialize(&mut deserializer)?;
        let type_info = partial_reflect.get_represented_type_info().unwrap();
        let type_path = type_info.type_path();
        let registration = ctx
            .registry
            .get(type_info.type_id())
            .ok_or_else(|| ErrorKind::Custom(format!("{type_path} is not registered")))?;
        let from_reflect = ctx
            .registry
            .get_type_data::<ReflectFromReflect>(registration.type_id())
            .unwrap_or_else(|| panic!("`{type_path}` should reflect `FromReflect`"));
        let reflect = from_reflect
            .from_reflect(&*partial_reflect)
            .ok_or_else(|| {
                ErrorKind::Custom(format!("unable to convert `{type_path}` into actual type"))
            })?;
        let reflect_actor = registration.data::<ReflectActorBundle>().ok_or_else(|| {
            ErrorKind::Custom(format!("`{type_path}` doesn't reflect `ActorBundle`"))
        })?;
        let actor = reflect_actor
            .get_boxed(reflect)
            .map_err(|_| ErrorKind::Custom(format!("`{type_path}` is not an `ActorBundle`")))?;
        actors.push(actor);
    }
    let select = DefaultOptions::new().deserialize_from(cursor)?;

    Ok(FamilyCreate {
        city_entity,
        scene: FamilyScene { name, actors },
        select,
    })
}

#[derive(SubStates, Component, Clone, Copy, Debug, Eq, Hash, PartialEq, EnumIter, Default)]
#[source(WorldState = WorldState::Family)]
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

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
#[require(
    Name,
    Budget,
    Replicated,
    FamilyMembers,
    StateScoped<GameState>(|| StateScoped(GameState::InGame))
)]
pub struct Family;

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize, Deref)]
#[reflect(Component)]
pub struct Budget(u32);

impl Default for Budget {
    fn default() -> Self {
        Self(20_000)
    }
}

/// Contains the entities of all the actors that belong to the family.
///
/// Automatically created and updated based on [`Actor`].
#[derive(Component, Default, Deref, DerefMut)]
pub struct FamilyMembers(Vec<Entity>);

/// Emitted when an actor spawned.
///
/// This additional level of indirection is needed because when an actor spawned from scene,
/// family entity might not have `FamilyMembers` yet.
#[derive(Event, Deref)]
struct FamilyMemberAdded(Entity);

/// Indicates locally controlled family.
///
/// Inserted automatically on [`ActiveActor`] insertion.
#[derive(Component)]
pub struct SelectedFamily;

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
