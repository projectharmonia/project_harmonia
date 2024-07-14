use bevy::prelude::*;
use itertools::Itertools;

use super::ObjectPath;
use crate::{
    asset::metadata,
    game_world::{actor::Actor, GameWorld},
    math::segment::Segment,
    navigation::NavPath,
};

pub(super) struct DoorPlugin;

impl Plugin for DoorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Door>()
            .add_systems(
                Update,
                (
                    Self::init,
                    (Self::update_passing_actors, Self::play_animation).chain(),
                )
                    .run_if(resource_exists::<GameWorld>),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup_passing_actors.run_if(resource_exists::<GameWorld>),
            );
    }
}

impl DoorPlugin {
    fn init(mut commands: Commands, objects: Query<Entity, Added<Door>>) {
        for entity in &objects {
            commands.entity(entity).insert(DoorState::default());
        }
    }

    /// Updates which actors going to intersect door via navigation paths.
    fn update_passing_actors(
        mut objects: Query<(&mut DoorState, &GlobalTransform, &Door)>,
        actors: Query<(Entity, &NavPath), Changed<NavPath>>,
    ) {
        for (actor_entity, nav_path) in &actors {
            // Remove from old passing actors.
            for (mut door_state, ..) in &mut objects {
                door_state.remove_passing(actor_entity);
            }

            for (nav_start, nav_end) in nav_path.iter().map(|point| point.xz()).tuple_windows() {
                let nav_segment = Segment::new(nav_start, nav_end);

                for (mut door_state, door_transform, door) in &mut objects {
                    let door_point = Vec3::X * door.half_width;
                    let door_start = door_transform.transform_point(door_point).xz();
                    let door_end = door_transform.transform_point(-door_point).xz();
                    let door_segment = Segment::new(door_start, door_end);
                    if nav_segment.intersects(door_segment) {
                        door_state.passing_actors.push(actor_entity);
                    }
                }
            }
        }
    }

    /// Plays animation for actors whose close to the door and going to intersect it.
    fn play_animation(
        mut animation_players: Query<&mut AnimationPlayer>,
        asset_server: Res<AssetServer>,
        animation_clips: Res<Assets<AnimationClip>>,
        children: Query<&Children>,
        actors: Query<&GlobalTransform>,
        mut objects: Query<(Entity, &GlobalTransform, &ObjectPath, &Door, &mut DoorState)>,
    ) {
        for (object_entity, object_transform, object_path, door, mut door_state) in &mut objects {
            let object_translation = object_transform.translation().xz();
            let should_open = door_state
                .passing_actors
                .iter()
                .filter_map(|&entity| actors.get(entity).ok())
                .map(|transform| transform.translation().xz().distance(object_translation))
                .any(|distance| distance < door.trigger_distance);

            if door_state.opened == should_open {
                continue;
            }

            if let Some(mut animation_player) = animation_players
                .iter_many_mut(children.iter_descendants(object_entity))
                .fetch_next()
            {
                if !door_state.animation_loaded {
                    let animation_path = metadata::gltf_asset(&object_path.0, "Animation0");
                    animation_player.play(asset_server.load(animation_path));
                    door_state.animation_loaded = true;
                }

                let speed = if should_open { 1.0 } else { -1.0 };
                animation_player.set_speed(speed);

                if animation_player.is_finished() {
                    // If animation in a finished state, it should be manually triggered again.
                    animation_player.replay();
                    if !should_open {
                        if let Some(clip) = animation_clips.get(animation_player.animation_clip()) {
                            animation_player.seek_to(clip.duration());
                        }
                    }
                }

                door_state.opened = should_open;
            }
        }
    }

    fn cleanup_passing_actors(
        mut removed_actors: RemovedComponents<Actor>,
        mut objects: Query<&mut DoorState>,
    ) {
        for entity in removed_actors.read() {
            for mut door_state in &mut objects {
                door_state.remove_passing(entity);
            }
        }
    }
}

/// Marks object as door.
///
/// Will trigger open animation when an actor passes through.
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct Door {
    half_width: f32,
    /// Distance on which animation will be triggered.
    ///
    /// Triggered only be actors that going to pass through.
    /// See also [`DoorState`]
    trigger_distance: f32,
}

/// Stores calculated information about the door.
#[derive(Component, Default)]
struct DoorState {
    animation_loaded: bool,
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
