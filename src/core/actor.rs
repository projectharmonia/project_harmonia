mod animation;
pub(super) mod movement;

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_replicon::prelude::*;
use derive_more::Display;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use strum::EnumIter;

use super::{
    asset_handles::{AssetCollection, AssetHandles},
    family::FamilySync,
    game_state::GameState,
    game_world::{parent_sync::ParentSync, AppIgnoreSavingExt, WorldState},
    task::TaskGroups,
};
use animation::{AnimationPlugin, HumanAnimation};
use movement::MovementPlugin;

pub(super) struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AnimationPlugin)
            .add_plugin(MovementPlugin)
            .replicate::<FirstName>()
            .replicate::<Sex>()
            .replicate::<LastName>()
            .replicate::<Players>()
            .not_replicate_if_present::<Name, FirstName>()
            .ignore_saving::<Players>()
            .add_mapped_client_event::<ActorSelect>()
            .add_client_event::<ActorDeselect>()
            .init_resource::<AssetHandles<Sex>>()
            .add_systems(
                (Self::init_system, Self::name_update_system).in_set(OnUpdate(WorldState::InWorld)),
            )
            .add_systems((
                Self::selection_system.in_set(OnUpdate(GameState::Family)),
                Self::deselection_system.in_schedule(OnExit(GameState::Family)),
            ))
            .add_systems(
                (
                    Self::selection_update_system,
                    Self::deselection_update_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl ActorPlugin {
    fn init_system(
        mut commands: Commands,
        human_models: Res<AssetHandles<Sex>>,
        human_animations: Res<AssetHandles<HumanAnimation>>,
        actors: Query<(Entity, &Sex), Changed<Sex>>,
    ) {
        for (entity, &sex) in &actors {
            commands
                .entity(entity)
                .insert((
                    VisibilityBundle::default(),
                    GlobalTransform::default(),
                    human_models.handle(sex),
                    human_animations.handle(HumanAnimation::Idle),
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
        mut select_events: EventWriter<ActorSelect>,
        activated_actors: Query<Entity, Added<ActiveActor>>,
    ) {
        if let Ok(entity) = activated_actors.get_single() {
            select_events.send(ActorSelect(entity));
        }
    }

    fn selection_update_system(
        mut commands: Commands,
        mut select_events: EventReader<FromClient<ActorSelect>>,
        mut actors: Query<(Entity, &mut Players)>,
    ) {
        for FromClient { client_id, event } in select_events.iter().copied() {
            // Remove previous.
            for (entity, mut players) in &mut actors {
                if let Some(index) = players.iter().position(|&id| id == client_id) {
                    if players.len() == 1 {
                        commands.entity(entity).remove::<Players>();
                    } else {
                        players.swap_remove(index);
                    }
                    break;
                }
            }

            if let Ok((_, mut players)) = actors.get_mut(event.0) {
                players.push(client_id);
            } else {
                commands
                    .entity(event.0)
                    .insert(Players(smallvec![client_id]));
            }
        }
    }

    fn deselection_system(
        mut commands: Commands,
        mut deselect_events: EventWriter<ActorDeselect>,
        active_actors: Query<Entity, With<ActiveActor>>,
    ) {
        commands
            .entity(active_actors.single())
            .remove::<ActiveActor>();
        deselect_events.send(ActorDeselect);
    }

    fn deselection_update_system(
        mut commands: Commands,
        mut deselect_events: EventReader<FromClient<ActorDeselect>>,
        mut actors: Query<(Entity, &mut Players)>,
    ) {
        for client_id in deselect_events.iter().map(|event| event.client_id) {
            for (entity, mut players) in &mut actors {
                if let Some(index) = players.iter().position(|&id| id == client_id) {
                    if players.len() == 1 {
                        commands.entity(entity).remove::<Players>();
                    } else {
                        players.swap_remove(index);
                    }
                    break;
                }
            }
        }
    }
}

#[derive(Clone, Component, Debug, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct FirstName(pub(crate) String);

#[derive(Clone, Component, Debug, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct LastName(pub(crate) String);

#[derive(
    Clone,
    Component,
    Copy,
    Debug,
    Default,
    Deserialize,
    EnumIter,
    PartialEq,
    Reflect,
    Serialize,
    IntoPrimitive,
)]
#[reflect(Component)]
#[repr(usize)]
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
}

impl AssetCollection for Sex {
    type AssetType = Scene;

    fn asset_path(&self) -> &'static str {
        match self {
            Sex::Male => "base/actors/bot/y_bot/y_bot.gltf#Scene0",
            Sex::Female => "base/actors/bot/x_bot/x_bot.gltf#Scene0",
        }
    }
}

/// Contains list of player IDs who controls this actor.
#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(crate) struct Players(SmallVec<[u64; 2]>);

/// Indicates locally controlled actor.
#[derive(Component)]
pub(crate) struct ActiveActor;

/// Selects a actor entity to play.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct ActorSelect(pub(crate) Entity);

impl MapEntities for ActorSelect {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct ActorDeselect;

/// Minimal actor components.
///
/// Used as a part of bigger bundles like [`PlayableActorBundle`] or [`EditableActorBundle`].
#[derive(Bundle, Debug, Deserialize, Serialize, Clone, Default)]
pub(crate) struct ActorBundle {
    pub(crate) first_name: FirstName,
    pub(crate) last_name: LastName,
    pub(crate) sex: Sex,
}

/// Components for a actor inside the game.
#[derive(Bundle)]
pub(super) struct PlayableActorBundle {
    family_sync: FamilySync,
    parent_sync: ParentSync,
    transform: Transform,
    task_groups: TaskGroups,
    replication: Replication,

    #[bundle]
    actor_bundle: ActorBundle,
}

impl PlayableActorBundle {
    pub(super) fn new(
        actor_bundle: ActorBundle,
        family_entity: Entity,
        city_entity: Entity,
    ) -> Self {
        Self {
            family_sync: FamilySync(family_entity),
            parent_sync: ParentSync(city_entity),
            transform: Default::default(),
            task_groups: TaskGroups::default(),
            replication: Replication,
            actor_bundle,
        }
    }
}
