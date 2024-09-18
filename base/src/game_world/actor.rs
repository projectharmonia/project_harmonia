mod animation_state;
pub(super) mod human;
pub mod needs;
pub mod task;

use avian3d::prelude::*;
use bevy::{
    asset::AssetPath,
    ecs::{entity::MapEntities, reflect::ReflectMapEntities},
    prelude::*,
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::{InheritOutlineBundle, OutlineBundle};
use bevy_replicon::prelude::*;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::{
    hover::{highlighting::OutlineHighlightingExt, Hoverable},
    WorldState,
};
use crate::{
    asset::collection::{AssetCollection, Collection},
    core::GameState,
};
use animation_state::{AnimationState, AnimationStatePlugin};
use human::HumanPlugin;
use needs::NeedsPlugin;
use task::TaskPlugin;

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
            .observe(Self::ensure_single_selection)
            .add_systems(OnExit(WorldState::Family), Self::remove_selection)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                SpawnScene,
                Self::init_children
                    .run_if(in_state(GameState::InGame))
                    .after(scene::scene_spawner_system),
            )
            .add_systems(
                PostUpdate,
                Self::update_names.run_if(in_state(GameState::InGame)),
            );
    }
}

pub const ACTOR_HEIGHT: f32 = 1.2;
pub const ACTOR_RADIUS: f32 = 0.4;

impl ActorPlugin {
    fn init(
        mut commands: Commands,
        actors: Query<Entity, (With<Actor>, Without<GlobalTransform>)>,
    ) {
        for entity in &actors {
            debug!("initializing actor `{entity}`");
            commands
                .entity(entity)
                .insert((
                    AnimationState::default(),
                    GlobalTransform::default(),
                    VisibilityBundle::default(),
                    OutlineBundle::highlighting(),
                    Hoverable,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        RigidBody::Kinematic,
                        SpatialBundle::from_transform(Transform::from_translation(
                            Vec3::Y * (ACTOR_HEIGHT / 2.0 + ACTOR_RADIUS),
                        )),
                        Collider::capsule(ACTOR_RADIUS, ACTOR_HEIGHT),
                    ));
                });
        }
    }

    fn init_children(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        actors: Query<Entity, With<Actor>>,
        children: Query<&Children>,
    ) {
        for actor_entity in actors.iter_many(ready_events.read().map(|event| event.parent)) {
            debug!("initializing outline for `{actor_entity}`");
            for child_entity in children.iter_descendants(actor_entity) {
                commands
                    .entity(child_entity)
                    .insert(InheritOutlineBundle::default());
            }
        }
    }

    fn update_names(
        mut commands: Commands,
        mut changed_names: Query<
            (Entity, &FirstName, &LastName),
            Or<(Changed<FirstName>, Changed<LastName>)>,
        >,
    ) {
        for (entity, first_name, last_name) in &mut changed_names {
            debug!("updating full name for `{entity}`");
            let mut entity = commands.entity(entity);
            entity.insert(Name::new(format!("{} {}", first_name.0, last_name.0)));
        }
    }

    fn remove_selection(mut commands: Commands, actors: Query<Entity, With<SelectedActor>>) {
        if let Ok(entity) = actors.get_single() {
            info!("deselecting actor `{entity}`");
            commands.entity(entity).remove::<SelectedActor>();
        }
    }

    fn ensure_single_selection(
        trigger: Trigger<OnAdd, SelectedActor>,
        mut commands: Commands,
        actors: Query<Entity, With<SelectedActor>>,
    ) {
        for actor_entity in actors.iter().filter(|&entity| entity != trigger.entity()) {
            debug!("deselecting previous actor `{actor_entity}`");
            commands.entity(actor_entity).remove::<SelectedActor>();
        }
    }
}

#[derive(Clone, Component, Default, Deref, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub struct FirstName(pub String);

#[derive(Clone, Component, Default, Deref, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub struct LastName(pub String);

#[derive(
    Display, Clone, EnumIter, Component, Copy, Default, Deserialize, PartialEq, Reflect, Serialize,
)]
#[reflect(Component)]
pub enum Sex {
    #[default]
    Male,
    Female,
}

/// Indicates locally controlled actor.
#[derive(Component)]
pub struct SelectedActor;

/// Marks entity as an actor.
#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component, MapEntities)]
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

#[reflect_trait]
pub trait ActorBundle: Reflect {
    #[allow(dead_code)]
    fn glyph(&self) -> &'static str;
}

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
