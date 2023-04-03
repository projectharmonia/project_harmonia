use bevy::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use crate::core::{
    asset_handles::{AssetCollection, AssetHandles},
    game_world::WorldState,
};

pub(super) struct HumanAnimationPlugin;

impl Plugin for HumanAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetHandles<HumanAnimation>>()
            .add_system(Self::play_animation_system.in_set(OnUpdate(WorldState::InWorld)));
    }
}

impl HumanAnimationPlugin {
    /// Plays animation after changing [`HumanAnimation`] or after [`Children`] initialization (to handle scene spawn delay).
    fn play_animation_system(
        human_animations: Res<AssetHandles<HumanAnimation>>,
        actors: Query<(Entity, &HumanAnimation), Or<(Changed<HumanAnimation>, Added<Children>)>>,
        children: Query<&Children>,
        mut animaption_players: Query<&mut AnimationPlayer>,
    ) {
        for (human_entity, &animation) in &actors {
            const MODEL_INDEX: usize = 1; // We assume that model spawns at second child (from 0).
            if let Some(model_entity) = children.iter_descendants(human_entity).nth(MODEL_INDEX) {
                let mut animation_player = animaption_players
                    .get_mut(model_entity)
                    .expect("human model should have animation player attached");
                animation_player
                    .play(human_animations.handle(animation))
                    .repeat();
            }
        }
    }
}

#[derive(Component, Clone, Copy, EnumIter, IntoPrimitive)]
#[repr(usize)]
pub(super) enum HumanAnimation {
    Idle,
}

impl AssetCollection for HumanAnimation {
    type AssetType = AnimationClip;

    fn asset_path(&self) -> &'static str {
        match self {
            HumanAnimation::Idle => "base/actors/animations/idle/idle.gltf#Animation0",
        }
    }
}
