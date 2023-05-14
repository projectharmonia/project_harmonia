use std::time::Duration;

use bevy::prelude::*;
use bevy_scene_hook::SceneHooked;

use crate::core::game_world::WorldState;

pub(super) struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::playing_system.in_set(OnUpdate(WorldState::InWorld)));
    }
}

impl AnimationPlugin {
    /// Plays animation after changing [`Handle<AnimationClip>`] or after [`SceneHooked`] insertion (to handle scene spawn delay).
    ///
    /// Makes the behavior similar to adding [`Handle<Scene>`].
    fn playing_system(
        actors: Query<
            (Entity, &Handle<AnimationClip>),
            Or<(Changed<Handle<AnimationClip>>, Added<SceneHooked>)>,
        >,
        children: Query<&Children>,
        mut animation_players: Query<&mut AnimationPlayer>,
    ) {
        for (entity, handle) in &actors {
            if let Some(mut animation_player) = animation_players
                .iter_many_mut(children.iter_descendants(entity))
                .fetch_next()
            {
                animation_player
                    .play_with_transition(handle.clone(), Duration::from_millis(200))
                    .repeat();
            }
        }
    }
}
