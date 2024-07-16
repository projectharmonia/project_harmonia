use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    animation_state::AnimationState,
    asset::collection::Collection,
    game_world::{
        actor::{ActorAnimation, Sex},
        GameWorld,
    },
    navigation::{NavPath, Navigation},
};

pub(super) struct MovementAnimationPlugin;

impl Plugin for MovementAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Movement>().add_systems(
            Update,
            Self::update_animation.run_if(resource_exists::<GameWorld>),
        );
    }
}

impl MovementAnimationPlugin {
    fn update_animation(
        actor_animations: Res<Collection<ActorAnimation>>,
        mut actors: Query<
            (Entity, &Sex, &Navigation, &NavPath, &mut AnimationState),
            Changed<NavPath>,
        >,
    ) {
        for (entity, sex, navigation, nav_path, mut animation_state) in &mut actors {
            if nav_path.is_empty() {
                debug!("playing idle animation for `{entity:?}`");
                animation_state.set_default(actor_animations.handle(ActorAnimation::Idle));
                continue;
            }

            let animation = match sex {
                Sex::Male => {
                    if navigation.speed() <= Movement::Walk.speed() {
                        ActorAnimation::MaleWalk
                    } else {
                        ActorAnimation::MaleRun
                    }
                }
                Sex::Female => {
                    if navigation.speed() <= Movement::Walk.speed() {
                        ActorAnimation::FemaleWalk
                    } else {
                        ActorAnimation::FemaleRun
                    }
                }
            };

            debug!("playing animation `{animation:?}` for `{entity:?}`");
            animation_state.set_default(actor_animations.handle(animation));
        }
    }
}

/// Triggers animation when the actor starts moving.
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
