mod move_here;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{ActorAnimation, Sex};
use crate::core::{
    asset_handles::AssetHandles,
    game_world::WorldName,
    navigation::{NavPath, Navigation},
};
use move_here::MoveHerePlugin;

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Movement>()
            .add_plugins(MoveHerePlugin)
            .add_systems(
                Update,
                Self::init_system.run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup_system.run_if(resource_exists::<WorldName>()),
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
    ) {
        for entity in &mut removed_navigations {
            commands.entity(entity).remove::<Movement>();
        }
    }
}

/// Setups navigation using the corresponding movement speed.
#[derive(Bundle)]
pub(super) struct MovementBundle {
    navigation: Navigation,
    movement: Movement,
}

impl MovementBundle {
    pub(super) fn new(movement: Movement) -> Self {
        Self {
            navigation: Navigation::new(movement.speed()),
            movement,
        }
    }

    pub(super) fn with_offset(mut self, offset: f32) -> Self {
        self.navigation = self.navigation.with_offset(offset);
        self
    }
}

/// Triggers animation when the actor starts moving.
#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
pub(super) enum Movement {
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
