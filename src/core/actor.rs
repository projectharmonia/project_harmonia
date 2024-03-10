pub(super) mod human;
mod movement_animation;
pub(crate) mod needs;
pub(crate) mod task;

use bevy::{
    prelude::*,
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::{InheritOutlineBundle, OutlineBundle};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::{
    asset::collection::{AssetCollection, Collection},
    game_state::GameState,
    game_world::WorldName,
    highlighting::OutlineHighlightingExt,
};
use crate::core::{animation_state::AnimationState, cursor_hover::CursorHoverable};
use human::HumanPlugin;
use movement_animation::MovementAnimationPlugin;
use needs::NeedsPlugin;
use task::TaskPlugin;

pub(super) struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Collection<ActorAnimation>>()
            .add_plugins((
                MovementAnimationPlugin,
                NeedsPlugin,
                HumanPlugin,
                TaskPlugin,
            ))
            .register_type::<Actor>()
            .register_type::<FirstName>()
            .register_type::<Sex>()
            .register_type::<LastName>()
            .replicate::<Actor>()
            .replicate::<FirstName>()
            .replicate::<Sex>()
            .replicate::<LastName>()
            .add_systems(OnExit(GameState::Family), Self::remove_selection)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>),
            )
            .add_systems(
                Update,
                Self::update_names.run_if(resource_exists::<WorldName>),
            )
            .add_systems(
                SpawnScene,
                Self::init_children
                    .run_if(resource_exists::<WorldName>)
                    .after(scene::scene_spawner_system),
            )
            .add_systems(PostUpdate, Self::ensure_single_selection);
    }
}

pub(crate) const ACTOR_HEIGHT: f32 = 1.2;
pub(crate) const ACTOR_RADIUS: f32 = 0.3;

impl ActorPlugin {
    fn init(
        mut commands: Commands,
        actor_animations: Res<Collection<ActorAnimation>>,
        actors: Query<Entity, Added<Actor>>,
    ) {
        for entity in &actors {
            commands
                .entity(entity)
                .insert((
                    AnimationState::new(actor_animations.handle(ActorAnimation::Idle)),
                    VisibilityBundle::default(),
                    GlobalTransform::default(),
                    OutlineBundle::highlighting(),
                    CursorHoverable,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        SpatialBundle::from_transform(Transform::from_translation(
                            Vec3::Y * (ACTOR_HEIGHT / 2.0 + ACTOR_RADIUS),
                        )),
                        Collider::capsule(ACTOR_HEIGHT, ACTOR_RADIUS),
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
            (Entity, Ref<FirstName>, Ref<LastName>),
            Or<(Changed<FirstName>, Changed<LastName>)>,
        >,
    ) {
        for (entity, first_name, last_name) in &mut changed_names {
            let mut entity = commands.entity(entity);
            entity.insert(Name::new(format!("{} {}", first_name.0, last_name.0)));
            if first_name.is_added() && last_name.is_added() {
                entity.dont_replicate::<Name>();
            }
        }
    }

    fn remove_selection(mut commands: Commands, actors: Query<Entity, With<SelectedActor>>) {
        if let Ok(entity) = actors.get_single() {
            commands.entity(entity).remove::<SelectedActor>();
        }
    }

    fn ensure_single_selection(
        mut commands: Commands,
        just_selected_actors: Query<Entity, Added<SelectedActor>>,
        actors: Query<Entity, With<SelectedActor>>,
    ) {
        if let Some(activated_entity) = just_selected_actors.iter().last() {
            for actor_entity in actors.iter().filter(|&entity| entity != activated_entity) {
                commands.entity(actor_entity).remove::<SelectedActor>();
            }
        }
    }
}

#[derive(Clone, Component, Default, Deref, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct FirstName(pub(crate) String);

#[derive(Clone, Component, Default, Deref, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct LastName(pub(crate) String);

#[derive(
    Display, Clone, EnumIter, Component, Copy, Default, Deserialize, PartialEq, Reflect, Serialize,
)]
#[reflect(Component)]
pub(crate) enum Sex {
    #[default]
    Male,
    Female,
}

/// Indicates locally controlled actor.
#[derive(Component)]
pub(crate) struct SelectedActor;

/// Marks entity as an actor.
#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct Actor;

#[reflect_trait]
pub(crate) trait ActorBundle: Reflect {
    fn glyph(&self) -> &'static str;
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
