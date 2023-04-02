use bevy::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use crate::core::game_world::WorldState;

pub(super) struct HumanAnimationPlugin;

impl Plugin for HumanAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HumanAnimationHandles>()
            .add_system(Self::play_animation_system.in_set(OnUpdate(WorldState::InWorld)));
    }
}

impl HumanAnimationPlugin {
    /// Plays animation after changing [`HumanAnimation`] or after [`Children`] initialization (to handle scene spawn delay).
    fn play_animation_system(
        human_animations: Res<HumanAnimationHandles>,
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
                let handle = human_animations[animation as usize].clone();
                animation_player.play(handle).repeat();
            }
        }
    }
}

#[derive(Component, Clone, Copy, EnumIter)]
pub(super) enum HumanAnimation {
    Idle,
}

impl HumanAnimation {
    fn asset_path(self) -> &'static str {
        match self {
            HumanAnimation::Idle => "base/actors/animations/idle/idle.gltf#Animation0",
        }
    }
}

#[derive(Resource, Deref)]
struct HumanAnimationHandles(Vec<Handle<AnimationClip>>);

impl FromWorld for HumanAnimationHandles {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let handles = HumanAnimation::iter()
            .map(|value| asset_server.load(value.asset_path()))
            .collect();
        Self(handles)
    }
}
