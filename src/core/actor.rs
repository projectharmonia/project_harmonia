mod friendly;
pub(super) mod movement;
pub(crate) mod needs;
pub(crate) mod race;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use derive_more::Display;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    asset_handles::{AssetCollection, AssetHandles},
    family::FamilySync,
    game_world::WorldState,
};
use friendly::FriendlyPlugins;
use movement::MovementPlugin;
use needs::NeedsPlugin;
use race::RacePlugins;

pub(super) struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetHandles<ActorAnimation>>()
            .add_plugins(RacePlugins)
            .add_plugins(FriendlyPlugins)
            .add_plugin(MovementPlugin)
            .add_plugin(NeedsPlugin)
            .replicate::<Actor>()
            .replicate::<FirstName>()
            .replicate::<Sex>()
            .replicate::<LastName>()
            .not_replicate_if_present::<Name, FirstName>()
            .add_system(Self::name_update_system.in_set(OnUpdate(WorldState::InWorld)));
    }
}

impl ActorPlugin {
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
    Display,
    Clone,
    EnumIter,
    Component,
    Copy,
    Debug,
    Default,
    Deserialize,
    PartialEq,
    Reflect,
    Serialize,
)]
#[reflect(Component)]
pub(crate) enum Sex {
    #[default]
    Male,
    Female,
}

/// Indicates locally controlled actor.
#[derive(Component)]
pub(crate) struct ActiveActor;

/// Minimal actor components without a race.
#[derive(Bundle)]
pub(super) struct ActorBundle {
    first_name: FirstName,
    last_name: LastName,
    sex: Sex,
    family_sync: FamilySync,
    parent_sync: ParentSync,
    transform: Transform,
    actor: Actor,
    replication: Replication,
}

impl ActorBundle {
    pub(super) fn new(actor_scene: ActorScene, family_entity: Entity, city_entity: Entity) -> Self {
        Self {
            first_name: actor_scene.first_name,
            last_name: actor_scene.last_name,
            sex: actor_scene.sex,
            family_sync: FamilySync(family_entity),
            parent_sync: ParentSync(city_entity),
            transform: Default::default(), // TODO: Get spawn position from world.
            actor: Actor,
            replication: Replication,
        }
    }
}

/// Marks entity as an actor.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct Actor;

/// Serializable actor scene for gallery.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ActorScene {
    pub(crate) first_name: FirstName,
    pub(crate) last_name: LastName,
    pub(crate) sex: Sex,
    pub(crate) race_name: String,
}

#[derive(Clone, Copy, EnumIter, IntoPrimitive)]
#[repr(usize)]
pub(super) enum ActorAnimation {
    Idle,
    MaleWalk,
    FemaleWalk,
    MaleRun,
    FemaleRun,
    TellSecret,
    ThoughtfulNod,
}

impl AssetCollection for ActorAnimation {
    type AssetType = AnimationClip;

    fn asset_path(&self) -> &'static str {
        match self {
            ActorAnimation::Idle => "base/actors/animations/idle.gltf#Animation0",
            ActorAnimation::MaleWalk => "base/actors/animations/male_walk.gltf#Animation0",
            ActorAnimation::FemaleWalk => "base/actors/animations/female_walk.gltf#Animation0",
            ActorAnimation::MaleRun => "base/actors/animations/male_run.gltf#Animation0",
            ActorAnimation::FemaleRun => "base/actors/animations/female_run.gltf#Animation0",
            ActorAnimation::TellSecret => "base/actors/animations/tell_secret.gltf#Animation0",
            ActorAnimation::ThoughtfulNod => {
                "base/actors/animations/thoughtful_nod.gltf#Animation0"
            }
        }
    }
}
