use std::time::Duration;

use bevy::prelude::*;
use bevy_scene_hook::SceneHooked;

use crate::core::game_world::WorldState;

pub(super) struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AnimationEnded>().add_systems(
            (Self::playing_system, Self::init_system, Self::update_system)
                .in_set(OnUpdate(WorldState::InWorld)),
        );
    }
}

impl AnimationPlugin {
    /// Plays animation after changing [`Handle<AnimationClip>`] or after [`SceneHooked`] insertion (to handle scene spawn delay).
    ///
    /// Makes the behavior similar to adding [`Handle<Scene>`].
    fn playing_system(
        mut commands: Commands,
        scenes: Query<
            (Entity, &Handle<AnimationClip>),
            Or<(Changed<Handle<AnimationClip>>, Added<SceneHooked>)>,
        >,
        children: Query<&Children>,
        mut animation_players: Query<&mut AnimationPlayer>,
    ) {
        for (entity, animation_handle) in &scenes {
            if let Some(mut animation_player) = animation_players
                .iter_many_mut(children.iter_descendants(entity))
                .fetch_next()
            {
                commands.entity(entity).remove::<AnimationTimer>();
                animation_player
                    .play_with_transition(animation_handle.clone(), Duration::from_millis(200))
                    .repeat();
            }
        }
    }

    /// Inserts [`AnimationTimer`] after animation loading.
    fn init_system(
        mut commands: Commands,
        animations: Res<Assets<AnimationClip>>,
        scenes: Query<(Entity, &Handle<AnimationClip>), Without<AnimationTimer>>,
    ) {
        for (entity, animation_handle) in &scenes {
            if let Some(animation) = animations.get(animation_handle) {
                commands
                    .entity(entity)
                    .insert(AnimationTimer::new(animation.duration()));
            }
        }
    }

    fn update_system(
        time: Res<Time>,
        mut end_events: EventWriter<AnimationEnded>,
        mut scenes: Query<(Entity, &mut AnimationTimer)>,
    ) {
        for (entity, mut timer) in &mut scenes {
            timer.tick(time.delta());
            if timer.just_finished() {
                end_events.send(AnimationEnded(entity));
            }
        }
    }
}

/// Tracks animation elapsed time to notify when it finishes.
#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

impl AnimationTimer {
    fn new(duration: f32) -> Self {
        Self(Timer::from_seconds(duration, TimerMode::Repeating))
    }
}

pub(super) struct AnimationEnded(pub(super) Entity);
