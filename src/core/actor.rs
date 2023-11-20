pub(super) mod human;
mod movement;
pub(crate) mod needs;
pub(crate) mod task;

use bevy::{prelude::*, scene::SceneInstanceReady};
use bevy_mod_outline::OutlineBundle;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use derive_more::Display;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    asset::collection::{AssetCollection, Collection},
    cursor_hover::OutlineHoverExt,
    game_state::GameState,
    game_world::WorldName,
};
use crate::core::{
    animation_state::AnimationState, collision_groups::LifescapeGroupsExt, cursor_hover::Hoverable,
};
use human::HumanPlugin;
use movement::MovementPlugin;
use needs::NeedsPlugin;
use task::TaskPlugin;

pub(super) struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Collection<ActorAnimation>>()
            .add_plugins((MovementPlugin, NeedsPlugin, HumanPlugin, TaskPlugin))
            .register_type::<Actor>()
            .register_type::<FirstName>()
            .register_type::<Sex>()
            .register_type::<LastName>()
            .replicate::<Actor>()
            .replicate::<FirstName>()
            .replicate::<Sex>()
            .replicate::<LastName>()
            .add_systems(OnExit(GameState::Family), Self::deactivation_system)
            .add_systems(
                PreUpdate,
                Self::init_system
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                Update,
                (Self::scene_init_system, Self::name_update_system)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::ignore_name_system.before(ServerSet::Send),
                    Self::exclusive_system,
                ),
            );
    }
}

impl ActorPlugin {
    fn init_system(
        mut commands: Commands,
        actor_animations: Res<Collection<ActorAnimation>>,
        actors: Query<Entity, Added<Actor>>,
    ) {
        for entity in &actors {
            const HALF_HEIGHT: f32 = 0.6;
            const RADIUS: f32 = 0.3;
            commands
                .entity(entity)
                .insert((
                    AnimationState::new(actor_animations.handle(ActorAnimation::Idle)),
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
        mut ready_events: EventReader<SceneInstanceReady>,
        actors: Query<Entity, With<Actor>>,
        chidlren: Query<&Children>,
        meshes: Query<(), With<Handle<Mesh>>>,
    ) {
        for actor_entity in ready_events
            .read()
            .filter_map(|event| actors.get(event.parent).ok())
        {
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

    fn deactivation_system(mut commands: Commands, actors: Query<Entity, With<ActiveActor>>) {
        if let Ok(entity) = actors.get_single() {
            commands.entity(entity).remove::<ActiveActor>();
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

    fn ignore_name_system(mut commands: Commands, actors: Query<Entity, Added<FirstName>>) {
        for entity in &actors {
            commands.entity(entity).insert(Ignored::<Name>::default());
        }
    }
}

#[derive(Clone, Component, Default, Deref, Deserialize, Display, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct FirstName(pub(crate) String);

#[derive(Clone, Component, Default, Deref, Deserialize, Display, Reflect, Serialize)]
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
pub(crate) struct ActiveActor;

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
