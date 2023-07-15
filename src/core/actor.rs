mod friendly;
pub(super) mod movement;
pub(crate) mod needs;
pub(crate) mod race;

use bevy::prelude::*;
use bevy_mod_outline::OutlineBundle;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use derive_more::Display;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    asset_handles::{AssetCollection, AssetHandles},
    cursor_hover::OutlineHoverExt,
    family::ActorFamily,
    game_world::WorldState,
    ready_scene::ReadyScene,
};
use crate::core::{collision_groups::LifescapeGroupsExt, cursor_hover::Hoverable};
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
            .add_systems(
                (
                    Self::init_system,
                    Self::scene_init_system,
                    Self::name_update_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
            )
            .add_system(Self::exclusive_system.in_base_set(CoreSet::PostUpdate));
    }
}

impl ActorPlugin {
    fn init_system(
        mut commands: Commands,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        actors: Query<Entity, Added<Actor>>,
    ) {
        for entity in &actors {
            const HALF_HEIGHT: f32 = 0.6;
            const RADIUS: f32 = 0.3;
            commands
                .entity(entity)
                .insert((
                    actor_animations.handle(ActorAnimation::Idle),
                    VisibilityBundle::default(),
                    GlobalTransform::default(),
                    Hoverable,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        SpatialBundle::from_transform(Transform::from_translation(
                            Vec3::Y * (HALF_HEIGHT + RADIUS),
                        )),
                        CollisionGroups::new(Group::ACTOR, Group::ALL),
                        Collider::capsule_y(HALF_HEIGHT, RADIUS),
                    ));
                });
        }
    }

    fn scene_init_system(
        mut commands: Commands,
        actors: Query<Entity, (Added<ReadyScene>, With<Actor>)>,
        chidlren: Query<&Children>,
        meshes: Query<(), With<Handle<Mesh>>>,
    ) {
        for actor_entity in &actors {
            for child_entity in chidlren
                .iter_descendants(actor_entity)
                .filter(|&entity| meshes.get(entity).is_ok())
            {
                commands.entity(child_entity).insert(OutlineBundle::hover());
            }
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

    fn exclusive_system(
        mut commands: Commands,
        activated_actors: Query<Entity, Added<ActiveActor>>,
        actors: Query<Entity, With<ActiveActor>>,
    ) {
        if let Some(activated_entity) = activated_actors.iter().last() {
            for actor_entity in actors.iter().filter(|&entity| entity != activated_entity) {
                commands.entity(actor_entity).remove::<ActiveActor>();
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
    actor_family: ActorFamily,
    parent_sync: ParentSync,
    transform: Transform,
    actor: Actor,
    replication: Replication,
}

impl ActorBundle {
    pub(super) fn new(family_entity: Entity) -> Self {
        Self {
            actor_family: ActorFamily(family_entity),
            parent_sync: Default::default(),
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
