mod animation;
pub(super) mod movement;
pub(crate) mod needs;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use derive_more::Display;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    asset_handles::{AssetCollection, AssetHandles},
    family::FamilySync,
    game_world::{parent_sync::ParentSync, WorldState},
};
use animation::{AnimationPlugin, HumanAnimation};
use movement::MovementPlugin;
use needs::{Bladder, Energy, Fun, Hunger, Hygiene, NeedsPlugin, Social};

pub(super) struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(AnimationPlugin)
            .add_plugin(MovementPlugin)
            .add_plugin(NeedsPlugin)
            .replicate::<Actor>()
            .replicate::<FirstName>()
            .replicate::<Sex>()
            .replicate::<LastName>()
            .not_replicate_if_present::<Name, FirstName>()
            .init_resource::<AssetHandles<Sex>>()
            .add_systems(
                (
                    Self::init_system,
                    Self::init_mesh_system,
                    Self::name_update_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
            );
    }
}

impl ActorPlugin {
    fn init_system(
        mut commands: Commands,
        human_animations: Res<AssetHandles<HumanAnimation>>,
        actors: Query<Entity, Added<Actor>>,
    ) {
        for entity in &actors {
            commands.entity(entity).insert((
                VisibilityBundle::default(),
                GlobalTransform::default(),
                human_animations.handle(HumanAnimation::Idle),
            ));
        }
    }

    fn init_mesh_system(
        mut commands: Commands,
        human_models: Res<AssetHandles<Sex>>,
        actors: Query<(Entity, &Sex), Changed<Sex>>,
    ) {
        for (entity, &sex) in &actors {
            commands
                .entity(entity)
                .insert(human_models.handle(sex))
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

/// Indicates locally controlled actor.
#[derive(Component)]
pub(crate) struct ActiveActor;

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
    hunger: Hunger,
    social: Social,
    hygiene: Hygiene,
    fun: Fun,
    energy: Energy,
    bladder: Bladder,
    actor: Actor,
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
            hunger: Default::default(),
            social: Default::default(),
            hygiene: Default::default(),
            fun: Default::default(),
            energy: Default::default(),
            bladder: Default::default(),
            actor: Actor,
            replication: Replication,
            actor_bundle,
        }
    }
}

/// Marks entity as an actor.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct Actor;
