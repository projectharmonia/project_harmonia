mod move_here;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{ActorAnimation, Sex};
use crate::core::{
    asset_handles::AssetHandles,
    game_world::WorldState,
    navigation::{NavPath, Navigation},
};
use move_here::MoveHerePlugin;

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MoveHerePlugin).add_systems(
            (Self::init_system, Self::cleanup_system).in_set(OnUpdate(WorldState::InWorld)),
        );
    }
}

impl MovementPlugin {
    fn init_system(
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<(&Sex, &Movement, &mut Handle<AnimationClip>), Added<NavPath>>,
    ) {
        for (sex, movement, mut animation_handle) in &mut actors {
            let animation = match (sex, movement) {
                (Sex::Male, Movement::Walk) => ActorAnimation::MaleWalk,
                (Sex::Female, Movement::Walk) => ActorAnimation::FemaleWalk,
                (Sex::Male, Movement::Run) => ActorAnimation::MaleRun,
                (Sex::Female, Movement::Run) => ActorAnimation::FemaleRun,
            };
            *animation_handle = actor_animations.handle(animation);
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<&mut Handle<AnimationClip>, With<Movement>>,
    ) {
        for entity in &mut removed_navigations {
            if let Ok(mut animation_handle) = actors.get_mut(entity) {
                *animation_handle = actor_animations.handle(ActorAnimation::Idle);
                commands.entity(entity).remove::<Movement>();
            }
        }
    }
}

#[derive(Bundle)]
struct MovementBundle {
    navigation: Navigation,
    movement: Movement,
}

impl MovementBundle {
    fn new(movement: Movement) -> Self {
        Self {
            navigation: Navigation::new(movement.speed()),
            movement,
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
enum Movement {
    #[default]
    Walk,
    Run,
}

impl Movement {
    fn speed(self) -> f32 {
        match self {
            Movement::Walk => 2.0,
            Movement::Run => 4.0,
        }
    }
}
