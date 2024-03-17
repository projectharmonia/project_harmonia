use std::time::Duration;

use bevy::{
    animation::RepeatAnimation,
    prelude::*,
    scene::{self, SceneInstanceReady},
};

use super::game_world::GameWorld;

pub(super) struct AnimationStatePlugin;

impl Plugin for AnimationStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<AnimationFinished>()
            .add_systems(
                SpawnScene,
                Self::init_children
                    .run_if(resource_exists::<GameWorld>)
                    .after(scene::scene_spawner_system),
            )
            .add_systems(
                PostUpdate,
                (Self::play, Self::finish).run_if(resource_exists::<GameWorld>),
            );
    }
}

impl AnimationStatePlugin {
    fn init_children(
        mut ready_events: EventReader<SceneInstanceReady>,
        scenes: Query<(Entity, &AnimationState)>,
        children: Query<&Children>,
        mut animation_players: Query<&mut AnimationPlayer>,
    ) {
        for (entity, animation_state) in
            scenes.iter_many(ready_events.read().map(|event| event.parent))
        {
            if let Some(mut animation_player) = animation_players
                .iter_many_mut(children.iter_descendants(entity))
                .fetch_next()
            {
                animation_state.apply(&mut animation_player);
            }
        }
    }

    fn play(
        scenes: Query<(Entity, &AnimationState), Changed<AnimationState>>,
        children: Query<&Children>,
        mut animation_players: Query<&mut AnimationPlayer>,
    ) {
        for (entity, animation_state) in &scenes {
            if let Some(mut animation_player) = animation_players
                .iter_many_mut(children.iter_descendants(entity))
                .fetch_next()
            {
                animation_state.apply(&mut animation_player);
            }
        }
    }

    fn finish(
        mut finish_events: EventWriter<AnimationFinished>,
        mut scenes: Query<(Entity, &mut AnimationState)>,
        children: Query<&Children>,
        mut animation_players: Query<&mut AnimationPlayer>,
    ) {
        for (entity, mut animation_state) in &mut scenes {
            if let Some(mut animation_player) = animation_players
                .iter_many_mut(children.iter_descendants(entity))
                .fetch_next()
            {
                if animation_player.is_finished() {
                    animation_state.handle = None;
                    animation_state.apply(&mut animation_player);

                    finish_events.send(AnimationFinished(entity));
                }
            }
        }
    }
}

/// Applies animation to the child [`AnimationPlayer`].
///
/// Always plays default animation on repeat with an option
/// to temporary override this animation with another one.
#[derive(Component)]
pub(super) struct AnimationState {
    /// Animation that plays if no other animation is playing.
    default_handle: Handle<AnimationClip>,

    /// Animation that overrides the default animation if set.
    handle: Option<Handle<AnimationClip>>,

    /// How may times to repeat the animation from `handle` field.
    repeat: RepeatAnimation,
}

impl AnimationState {
    pub(super) fn new(default_handle: Handle<AnimationClip>) -> Self {
        Self {
            default_handle,
            handle: None,
            repeat: RepeatAnimation::Never,
        }
    }

    pub(super) fn set_default(&mut self, default_handle: Handle<AnimationClip>) {
        self.default_handle = default_handle;
    }

    pub(super) fn stop(&mut self) {
        self.handle = None;
    }

    pub(super) fn repeat(&mut self, handle: Handle<AnimationClip>) {
        self.handle = Some(handle);
        self.repeat = RepeatAnimation::Forever;
    }

    pub(super) fn play_once(&mut self, handle: Handle<AnimationClip>) {
        self.handle = Some(handle);
        self.repeat = RepeatAnimation::Never;
    }

    fn apply(&self, animation_player: &mut AnimationPlayer) {
        const TRANSITION_TIME: Duration = Duration::from_millis(200);

        if let Some(handle) = &self.handle {
            animation_player
                .play_with_transition(handle.clone(), TRANSITION_TIME)
                .set_repeat(self.repeat);
        } else {
            animation_player
                .play_with_transition(self.default_handle.clone(), TRANSITION_TIME)
                .set_repeat(RepeatAnimation::Forever);
        }
    }
}

#[derive(Event)]
pub(super) struct AnimationFinished(pub(super) Entity);
