mod movement;

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use strum::EnumIter;

use super::{
    family::FamilySync,
    game_state::GameState,
    game_world::{parent_sync::ParentSync, AppIgnoreSavingExt, WorldState},
    network::{
        network_event::client_event::{ClientEvent, ClientEventAppExt},
        replication::replication_rules::{AppReplicationExt, Replication},
        sets::NetworkSet,
    },
    task::TaskQueue,
};
use movement::MovementPlugin;

pub(super) struct DollPlugin;

impl Plugin for DollPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MovementPlugin)
            .register_and_replicate::<FirstName>()
            .register_and_replicate::<Sex>()
            .register_and_replicate::<LastName>()
            .register_and_replicate::<DollPlayers>()
            .not_replicate_if_present::<Name, FirstName>()
            .ignore_saving::<DollPlayers>()
            .add_mapped_client_event::<DollSelect>()
            .add_client_event::<DollDeselect>()
            .add_systems(
                (Self::init_system, Self::name_update_system).in_set(OnUpdate(WorldState::InWorld)),
            )
            .add_systems(
                (Self::selection_system, Self::deselection_system)
                    .in_set(OnUpdate(GameState::Family)),
            )
            .add_systems(
                (
                    Self::selection_update_system,
                    Self::deselection_update_system,
                )
                    .in_set(NetworkSet::Authoritve),
            )
            .add_system(Self::deactivation_system.in_schedule(OnExit(GameState::Family)));
    }
}

impl DollPlugin {
    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        new_dolls: Query<(Entity, &Sex), Changed<Sex>>,
    ) {
        for (entity, &sex) in &new_dolls {
            let scene_handle: Handle<Scene> = asset_server.load(sex.model_path());
            commands
                .entity(entity)
                .insert((
                    VisibilityBundle::default(),
                    GlobalTransform::default(),
                    scene_handle,
                ))
                .despawn_descendants();
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
        mut select_events: EventWriter<DollSelect>,
        activated_dolls: Query<Entity, Added<ActiveDoll>>,
    ) {
        if let Ok(entity) = activated_dolls.get_single() {
            select_events.send(DollSelect(entity));
        }
    }

    fn selection_update_system(
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

    fn deselection_system(
        mut deselect_events: EventWriter<DollDeselect>,
        mut deactivated_dolls: RemovedComponents<ActiveDoll>,
    ) {
        if deactivated_dolls.iter().count() != 0 {
            deselect_events.send(DollDeselect);
        }
    }

    fn deselection_update_system(
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
}

#[derive(Clone, Component, Debug, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct FirstName(pub(crate) String);

#[derive(Clone, Component, Debug, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct LastName(pub(crate) String);

#[derive(
    Clone, Component, Copy, Debug, Default, Deserialize, EnumIter, PartialEq, Reflect, Serialize,
)]
#[reflect(Component)]
pub(crate) enum Sex {
    #[default]
    Male,
    Female,
}

impl Sex {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Sex::Male => "♂",
            Sex::Female => "♀",
        }
    }

    fn model_path(self) -> &'static str {
        match self {
            Sex::Male => "base/dolls/bot/y_bot/y_bot.gltf#Scene0",
            Sex::Female => "base/dolls/bot/x_bot/x_bot.gltf#Scene0",
        }
    }
}

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

/// Minimal doll components.
///
/// Used as a part of bigger bundles like [`PlayableDollBundle`] or [`EditableDollBundle`].
#[derive(Bundle, Debug, Deserialize, Serialize, Clone, Default)]
pub(crate) struct DollBundle {
    pub(crate) first_name: FirstName,
    pub(crate) last_name: LastName,
    pub(crate) sex: Sex,
}

/// Components for a doll inside the game.
#[derive(Bundle)]
pub(super) struct PlayableDollBundle {
    family_sync: FamilySync,
    parent_sync: ParentSync,
    transform: Transform,
    task_queue: TaskQueue,
    replication: Replication,

    #[bundle]
    doll_bundle: DollBundle,
}

impl PlayableDollBundle {
    pub(super) fn new(doll_bundle: DollBundle, family_entity: Entity, city_entity: Entity) -> Self {
        Self {
            family_sync: FamilySync(family_entity),
            parent_sync: ParentSync(city_entity),
            transform: Default::default(),
            task_queue: Default::default(),
            replication: Replication,
            doll_bundle,
        }
    }
}
