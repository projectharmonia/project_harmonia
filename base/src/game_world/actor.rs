mod animation_state;
pub(super) mod human;
pub mod needs;
pub mod task;

use std::fmt::Write;

use avian3d::prelude::*;
use bevy::{
    asset::AssetPath,
    ecs::{entity::MapEntities, reflect::ReflectMapEntities},
    prelude::*,
};
use bevy_mod_outline::OutlineVolume;
use bevy_replicon::prelude::*;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    family::editor::{EditorFirstName, EditorLastName, EditorSex},
    highlighting::HIGHLIGHTING_VOLUME,
    navigation::Navigation,
    Layer, WorldState,
};
use crate::{
    asset::collection::{AssetCollection, Collection},
    core::GameState,
};
use animation_state::{AnimationState, AnimationStatePlugin};
use human::HumanPlugin;
use needs::NeedsPlugin;
use task::{TaskGroups, TaskPlugin};

pub(super) struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Collection<ActorAnimation>>()
            .add_plugins((AnimationStatePlugin, NeedsPlugin, HumanPlugin, TaskPlugin))
            .register_type::<Transform>()
            .register_type::<Actor>()
            .register_type::<FirstName>()
            .register_type::<Sex>()
            .register_type::<LastName>()
            .register_type::<Movement>()
            .replicate_mapped::<Actor>()
            .replicate::<FirstName>()
            .replicate::<Sex>()
            .replicate::<LastName>()
            .add_systems(
                OnExit(WorldState::Family),
                Self::remove_selection.never_param_warn(),
            )
            .add_systems(
                PostUpdate,
                Self::update_names.run_if(in_state(GameState::InGame)),
            );
    }
}

const ACTOR_HEIGHT: f32 = 1.8;
pub(super) const ACTOR_RADIUS: f32 = 0.4;

impl ActorPlugin {
    fn update_names(
        mut changed_names: Query<
            (Entity, &FirstName, &LastName, &mut Name),
            Or<(Changed<FirstName>, Changed<LastName>)>,
        >,
    ) {
        for (entity, first_name, last_name, mut name) in &mut changed_names {
            debug!("updating full name for `{entity}`");
            name.mutate(|name| {
                name.clear();
                write!(name, "{} {}", first_name.0, last_name.0).unwrap();
            });
        }
    }

    fn remove_selection(
        mut commands: Commands,
        selected_entity: Single<Entity, With<SelectedActor>>,
    ) {
        info!("deselecting actor `{}`", *selected_entity);
        commands.entity(*selected_entity).remove::<SelectedActor>();
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub struct FirstName(pub String);

impl From<EditorFirstName> for FirstName {
    fn from(value: EditorFirstName) -> Self {
        Self(value.0)
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub struct LastName(pub String);

impl From<EditorLastName> for LastName {
    fn from(value: EditorLastName) -> Self {
        Self(value.0)
    }
}

#[derive(Clone, Component, Copy, Default, Deserialize, PartialEq, Reflect, Serialize, Debug)]
#[reflect(Component)]
pub enum Sex {
    #[default]
    Male,
    Female,
}

impl From<EditorSex> for Sex {
    fn from(value: EditorSex) -> Self {
        match value {
            EditorSex::Male => Self::Male,
            EditorSex::Female => Self::Female,
        }
    }
}

/// Indicates locally controlled actor.
#[derive(Component)]
pub struct SelectedActor;

/// Marks entity as an actor.
#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component, MapEntities)]
#[require(
    FirstName,
    LastName,
    Sex,
    Replicated,
    ParentSync,
    Navigation,
    Name,
    AnimationState,
    SceneRoot,
    ActorTaskGroups,
    RigidBody(|| RigidBody::Kinematic),
    Collider(|| Collider::capsule_endpoints(
        ACTOR_RADIUS,
        Vec3::Y * ACTOR_RADIUS,
        Vec3::Y * (ACTOR_HEIGHT - ACTOR_RADIUS),
    )),
    CollisionLayers(|| CollisionLayers::new(
        Layer::Actor,
        LayerMask::NONE,
    )),
    OutlineVolume(|| HIGHLIGHTING_VOLUME)
)]
pub struct Actor {
    pub family_entity: Entity,
}

impl FromWorld for Actor {
    fn from_world(_world: &mut World) -> Self {
        Self {
            family_entity: Entity::PLACEHOLDER,
        }
    }
}

impl MapEntities for Actor {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.family_entity = entity_mapper.map_entity(self.family_entity);
    }
}

#[derive(Component, Default, Deref, DerefMut)]
struct ActorTaskGroups(TaskGroups);

#[derive(Clone, Copy, Debug, EnumIter, IntoPrimitive)]
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

    fn asset_path(&self) -> AssetPath<'static> {
        match self {
            ActorAnimation::Idle => {
                GltfAssetLabel::Animation(0).from_asset("base/actors/animations/idle.gltf")
            }
            ActorAnimation::MaleWalk => {
                GltfAssetLabel::Animation(0).from_asset("base/actors/animations/male_walk.gltf")
            }
            ActorAnimation::FemaleWalk => {
                GltfAssetLabel::Animation(0).from_asset("base/actors/animations/female_walk.gltf")
            }
            ActorAnimation::MaleRun => {
                GltfAssetLabel::Animation(0).from_asset("base/actors/animations/male_run.gltf")
            }
            ActorAnimation::FemaleRun => {
                GltfAssetLabel::Animation(0).from_asset("base/actors/animations/female_run.gltf")
            }
            ActorAnimation::TellSecret => {
                GltfAssetLabel::Animation(0).from_asset("base/actors/animations/tell_secret.gltf")
            }
            ActorAnimation::ThoughtfulNod => GltfAssetLabel::Animation(0)
                .from_asset("base/actors/animations/thoughtful_nod.gltf"),
        }
    }
}

/// Type of actor movement.
#[derive(Clone, Copy, Default, Deserialize, Reflect, Serialize)]
pub(super) enum Movement {
    #[default]
    Walk,
    Run,
}

impl Movement {
    pub(super) fn speed(self) -> f32 {
        match self {
            Movement::Walk => 2.0,
            Movement::Run => 4.0,
        }
    }
}
