use std::time::Duration;

use bevy::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use crate::core::{
    asset_handles::{AssetCollection, AssetHandles},
    game_world::WorldState,
};

pub(super) struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetHandles<ActorAnimation>>()
            .add_system(Self::playing_system.in_set(OnUpdate(WorldState::InWorld)));
    }
}

impl AnimationPlugin {
    /// Plays animation after changing [`Handle<AnimationClip>`] or after [`Children`] initialization (to handle scene spawn delay).
    ///
    /// Makes the behavior similar to adding [`Handle<Scene>`].
    fn playing_system(
        actors: Query<
            (Entity, &Handle<AnimationClip>),
            Or<(Changed<Handle<AnimationClip>>, Added<Children>)>,
        >,
        children: Query<&Children>,
        mut animaption_players: Query<&mut AnimationPlayer>,
    ) {
        for (human_entity, handle) in &actors {
            if let Some(mut animation_player) = animaption_players
                .iter_many_mut(children.iter_descendants(human_entity))
                .fetch_next()
            {
                animation_player
                    .play_with_transition(handle.clone(), Duration::from_millis(200))
                    .repeat();
            }
        }
    }
}

#[derive(Clone, Copy, EnumIter, IntoPrimitive)]
#[repr(usize)]
pub(super) enum ActorAnimation {
    Idle,
    MaleWalk,
    FemaleWalk,
}

impl AssetCollection for ActorAnimation {
    type AssetType = AnimationClip;

    fn asset_path(&self) -> &'static str {
        match self {
            ActorAnimation::Idle => "base/actors/animations/idle/idle.gltf#Animation0",
            ActorAnimation::MaleWalk => {
                "base/actors/animations/male_walk/male_walk.gltf#Animation0"
            }
            ActorAnimation::FemaleWalk => {
                "base/actors/animations/female_walk/female_walk.gltf#Animation0"
            }
        }
    }
}
