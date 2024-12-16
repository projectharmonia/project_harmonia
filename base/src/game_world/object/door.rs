use std::path::Path;

use bevy::{asset::AssetPath, prelude::*};
use itertools::Itertools;

use crate::{
    asset::{
        self,
        info::{MapPaths, ReflectMapPaths},
    },
    core::GameState,
    game_world::{actor::Actor, navigation::NavPath, segment::Segment},
};

pub(super) struct DoorPlugin;

impl Plugin for DoorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Door>()
            .observe(Self::cleanup_passing_actors)
            .add_systems(
                Update,
                (
                    Self::init,
                    (Self::update_passing_actors, Self::play_animation).chain(),
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl DoorPlugin {
    fn init(mut commands: Commands, objects: Query<Entity, (With<Door>, Without<DoorState>)>) {
        for entity in &objects {
            debug!("initializing door for `{entity}`");
            commands.entity(entity).insert(DoorState::default());
        }
    }

    /// Updates which actors going to intersect door via navigation paths.
    fn update_passing_actors(
        mut doors: Query<(&Parent, &mut DoorState, &Transform, &Door)>,
        actors: Query<(Entity, &Parent, &NavPath), Changed<NavPath>>,
    ) {
        for (actor_entity, actor_parent, path) in &actors {
            // Remove from old passing actors.
            for (door_parent, mut door_state, ..) in &mut doors {
                if actor_parent == door_parent {
                    door_state.remove_passing(actor_entity);
                }
            }

            for (nav_start, nav_end) in path.iter().map(|point| point.xz()).tuple_windows() {
                let nav_segment = Segment::new(nav_start, nav_end);

                for (door_parent, mut door_state, door_transform, door) in &mut doors {
                    if actor_parent != door_parent {
                        continue;
                    }

                    let door_point = Vec3::X * door.half_width;
                    let door_start = door_transform.transform_point(door_point).xz();
                    let door_end = door_transform.transform_point(-door_point).xz();
                    let door_segment = Segment::new(door_start, door_end);
                    if nav_segment.intersects(door_segment) {
                        debug!("marking path of actor `{actor_entity}` as passing");
                        door_state.passing_actors.push(actor_entity);
                    }
                }
            }
        }
    }

    /// Plays animation for actors whose close to the door and going to intersect it.
    fn play_animation(
        mut commands: Commands,
        mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
        asset_server: Res<AssetServer>,
        mut graphs: ResMut<Assets<AnimationGraph>>,
        children: Query<&Children>,
        actors: Query<(&Parent, &Transform)>,
        mut objects: Query<(Entity, &Parent, &Transform, &Door, &mut DoorState)>,
    ) {
        for (object_entity, object_parent, object_transform, door, mut door_state) in &mut objects {
            let object_translation = object_transform.translation.xz();
            let should_open = door_state
                .passing_actors
                .iter()
                .filter_map(|&entity| actors.get(entity).ok())
                .filter(|(parent, _)| *parent == object_parent)
                .map(|(_, transform)| transform.translation.xz().distance(object_translation))
                .any(|distance| distance < door.trigger_distance);

            if door_state.opened == should_open {
                continue;
            }

            if let Some((entity, mut animation_player)) = animation_players
                .iter_many_mut(children.iter_descendants(object_entity))
                .fetch_next()
            {
                let speed = if should_open { 1.0 } else { -1.0 };
                if let Some(animation_index) = door_state.animation_index {
                    let animation = animation_player
                        .animation_mut(animation_index)
                        .expect("open animation should be added");
                    animation.set_speed(speed);
                    if animation.is_finished() {
                        animation.replay();
                        if !should_open {
                            // HACK: seek to the end by passing a big value.
                            // Necessary to play backwards.
                            animation.seek_to(20.0);
                        }
                    }
                } else {
                    debug!(
                        "initializing open animation '{}' for `{object_entity}`",
                        door.open_animation
                    );

                    let (graph, animation_index) =
                        AnimationGraph::from_clip(asset_server.load(door.open_animation.clone()));
                    commands.entity(entity).insert(graphs.add(graph));
                    door_state.animation_index = Some(animation_index);
                    animation_player.play(animation_index).set_speed(speed);
                }

                debug!("playing open animation with speed {speed}");
                door_state.opened = should_open;
            }
        }
    }

    fn cleanup_passing_actors(
        trigger: Trigger<OnRemove, Actor>,
        mut objects: Query<&mut DoorState>,
    ) {
        for mut door_state in &mut objects {
            debug!("removing path of deleted actor `{}`", trigger.entity());
            door_state.remove_passing(trigger.entity());
        }
    }
}

/// Marks object as door.
///
/// Will trigger open animation when an actor passes through.
#[derive(Component, Reflect, Default)]
#[reflect(Component, MapPaths)]
pub(crate) struct Door {
    half_width: f32,
    /// Distance on which animation will be triggered.
    ///
    /// Triggered only be actors that going to pass through.
    /// See also [`DoorState`]
    trigger_distance: f32,
    open_animation: AssetPath<'static>,
}

impl MapPaths for Door {
    fn map_paths(&mut self, dir: &Path) {
        asset::change_parent_dir(&mut self.open_animation, dir);
    }
}

/// Stores calculated information about the door.
#[derive(Component, Default)]
struct DoorState {
    animation_index: Option<AnimationNodeIndex>,
    opened: bool,

    /// Actors whose navigation paths intersect this door.
    passing_actors: Vec<Entity>,
}

impl DoorState {
    fn remove_passing(&mut self, actor_entity: Entity) {
        if let Some(index) = self
            .passing_actors
            .iter()
            .position(|&entity| entity == actor_entity)
        {
            self.passing_actors.remove(index);
        }
    }
}
