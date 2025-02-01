use bevy::{animation::RepeatAnimation, prelude::*, scene::SceneInstanceReady, utils::Duration};
use strum::EnumCount;

use super::{ActorAnimation, Movement, Sex};
use crate::{
    asset::collection::Collection,
    core::GameState,
    game_world::navigation::{NavPath, Navigation},
};

pub(super) struct AnimationStatePlugin;

impl Plugin for AnimationStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(init_scene)
            .add_systems(PostUpdate, update.run_if(in_state(GameState::InGame)));
    }
}

fn init_scene(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    actor_animations: Res<Collection<ActorAnimation>>,
    mut actors: Query<(&mut AnimationState, &Sex)>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
) {
    let Ok((mut state, sex)) = actors.get_mut(trigger.entity()) else {
        return;
    };

    if let Some((player_entity, mut player)) = players
        .iter_many_mut(children.iter_descendants(trigger.entity()))
        .fetch_next()
    {
        debug!(
            "initializing player `{player_entity}` for state `{}`",
            trigger.entity()
        );

        let mut graph = AnimationGraph::new();
        let idle_handle = actor_animations.handle(ActorAnimation::Idle);
        let walk_handle = match sex {
            Sex::Male => actor_animations.handle(ActorAnimation::MaleWalk),
            Sex::Female => actor_animations.handle(ActorAnimation::FemaleWalk),
        };
        let run_handle = match sex {
            Sex::Male => actor_animations.handle(ActorAnimation::MaleRun),
            Sex::Female => actor_animations.handle(ActorAnimation::FemaleRun),
        };

        state.nodes[AnimationNode::Idle as usize] = graph.add_clip(idle_handle, 1.0, graph.root);
        state.nodes[AnimationNode::Walk as usize] = graph.add_clip(walk_handle, 1.0, graph.root);
        state.nodes[AnimationNode::Run as usize] = graph.add_clip(run_handle, 1.0, graph.root);
        state.nodes[AnimationNode::Montage as usize] = graph.add_blend(1.0, graph.root);
        state.player_entity = Some(player_entity);

        let mut transitions = AnimationTransitions::new();
        transitions.play(
            &mut player,
            state.nodes[AnimationNode::Idle as usize],
            Duration::ZERO,
        );

        commands.entity(player_entity).insert((
            transitions,
            AnimationGraphHandle(animation_graphs.add(graph)),
        ));
    }
}

fn update(
    mut commands: Commands,
    mut actors: Query<(Entity, &mut AnimationState, &Navigation, Ref<NavPath>)>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &AnimationGraphHandle,
    )>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (actor_entity, mut state, navigation, path) in &mut actors {
        let Some(player_entity) = state.player_entity else {
            continue;
        };
        let Ok((mut player, mut transitions, handle)) = players.get_mut(player_entity) else {
            continue;
        };

        match &state.montage_state {
            MontageState::Stopped => trace!("no montage to play"),
            MontageState::Pending(montage) => {
                debug!("applying pending montage for `{actor_entity}`");
                let graph = graphs
                    .get_mut(handle)
                    .expect("animation graph handle should be valid");
                let index = state.nodes[AnimationNode::Montage as usize];
                let node = graph.get_mut(index).expect("montage index should be valid");
                node.node_type = AnimationNodeType::Clip(montage.handle.clone());

                transitions
                    .play(&mut player, index, montage.transition_time)
                    .set_repeat(montage.repeat);
                state.current_node = AnimationNode::Montage;
                state.montage_state = MontageState::Playing;
                continue;
            }
            MontageState::Playing => {
                let index = state.nodes[AnimationNode::Montage as usize];
                let animation = player.animation(index).expect("montage should be active");
                if !animation.is_finished() {
                    trace!("playing montage for `{actor_entity}`");
                    // Continue playing, nothing to update.
                    continue;
                }

                debug!("finished montage for `{actor_entity}`");
                commands.trigger_targets(MontageFinished, actor_entity);
                state.montage_state = MontageState::Stopped;
            }
        }

        let node = if path.is_empty() {
            AnimationNode::Idle
        } else if navigation.speed() <= Movement::Walk.speed() {
            AnimationNode::Walk
        } else {
            AnimationNode::Run
        };

        if state.current_node != node {
            debug!("switching current node to `{node:?}` for `{actor_entity}`");
            let index = state.nodes[node as usize];
            transitions
                .play(&mut player, index, DEFAULT_TRANSITION_TIME)
                .set_repeat(RepeatAnimation::Forever);

            state.current_node = node;
        }
    }
}

const DEFAULT_TRANSITION_TIME: Duration = Duration::from_millis(200);

/// Manages actor animations based on the current state.
///
/// State animations are driven by the actor's navigation speed.
/// State animations can be temporarily overridden by a montage.
#[derive(Component, Default)]
pub(super) struct AnimationState {
    current_node: AnimationNode,
    nodes: [AnimationNodeIndex; AnimationNode::COUNT],
    montage_state: MontageState,
    player_entity: Option<Entity>,
}

impl AnimationState {
    /// Temporarily overrides the current animation state with a montage.
    ///
    /// Triggers [`MontageFinished`] when the montage completes,
    /// then resumes the animation based on the current state.
    pub(super) fn play_montage(&mut self, montage: Montage) {
        self.montage_state = MontageState::Pending(montage);
    }

    /// Stops the current montage, if any.
    ///
    /// Resumes the animation based on the current state.
    pub(super) fn stop_montage(&mut self) {
        self.montage_state = MontageState::Stopped;
    }
}

#[derive(Default)]
enum MontageState {
    #[default]
    Stopped,
    Pending(Montage),
    Playing,
}

#[derive(Event)]
pub(super) struct Montage {
    handle: Handle<AnimationClip>,
    repeat: RepeatAnimation,
    transition_time: Duration,
}

impl Montage {
    pub(super) fn new(handle: Handle<AnimationClip>) -> Self {
        Self {
            handle,
            repeat: RepeatAnimation::Count(1),
            transition_time: DEFAULT_TRANSITION_TIME,
        }
    }

    pub(super) fn with_repeat(mut self, repeat: RepeatAnimation) -> Self {
        self.repeat = repeat;
        self
    }
}

#[derive(Event)]
pub(super) struct MontageFinished;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, EnumCount)]
enum AnimationNode {
    #[default]
    Idle,
    Walk,
    Run,
    Montage,
}
