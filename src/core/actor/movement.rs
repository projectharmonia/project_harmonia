use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    actor::{ActorAnimation, Sex},
    asset_handles::AssetHandles,
    game_world::WorldName,
    navigation::{NavPath, Navigation},
};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Movement>().add_systems(
            Update,
            Self::init_system.run_if(resource_exists::<WorldName>()),
        );
    }
}

impl MovementPlugin {
    fn init_system(
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<(&Sex, &Navigation, &mut Handle<AnimationClip>), Added<NavPath>>,
    ) {
        for (sex, navigation, mut animation_handle) in &mut actors {
            let animation = match sex {
                Sex::Male => {
                    if navigation.speed <= Movement::Walk.speed() {
                        ActorAnimation::MaleWalk
                    } else {
                        ActorAnimation::MaleRun
                    }
                }
                Sex::Female => {
                    if navigation.speed <= Movement::Walk.speed() {
                        ActorAnimation::FemaleWalk
                    } else {
                        ActorAnimation::FemaleRun
                    }
                }
            };

            *animation_handle = actor_animations.handle(animation);
        }
    }
}

/// Triggers animation when the actor starts moving.
#[derive(Clone, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
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
