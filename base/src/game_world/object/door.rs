use bevy::prelude::*;
use itertools::Itertools;

use super::ObjectMeta;
use crate::{
    asset::metadata::object_metadata::ObjectMetadata, core::GameState, game_world::actor::Actor,
    math::segment::Segment, navigation::NavPath,
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
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup_passing_actors.run_if(in_state(GameState::InGame)),
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
        object_metadata: Res<Assets<ObjectMetadata>>,
        mut graphs: ResMut<Assets<AnimationGraph>>,
        children: Query<&Children>,
        actors: Query<&GlobalTransform>,
        mut objects: Query<(Entity, &GlobalTransform, &ObjectMeta, &Door, &mut DoorState)>,
    ) {
        for (object_entity, object_transform, object_meta, door, mut door_state) in &mut objects {
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

            if let Some((entity, mut animation_player)) = animation_players
                .iter_many_mut(children.iter_descendants(object_entity))
                .fetch_next()
            {
                let speed = if should_open { 1.0 } else { -1.0 };
                if let Some(animation_index) = door_state.animation_index {
                    if animation_player.is_playing_animation(animation_index) {
                        // If the animation in a finished state, it should be manually triggered again.
                        let active_animation = animation_player
                            .animation_mut(animation_index)
                            .expect("open animation should be always active");
                        active_animation.replay();
                        if !should_open {
                            // HACK: seek to the end by passing a big value.
                            active_animation.seek_to(20.0);
                        }
                    }
                } else {
                    let metadata_handle = asset_server
                        .get_handle(&object_meta.0)
                        .expect("metadata should be preloaded");
                    let metadata = object_metadata.get(&metadata_handle).unwrap();

                    let animation_path =
                        GltfAssetLabel::Animation(0).from_asset(metadata.general.asset.clone());
                    debug!("initializing open animation '{animation_path}' for `{object_entity}`");

                    let (graph, animation_index) =
                        AnimationGraph::from_clip(asset_server.load(animation_path));
                    commands.entity(entity).insert(graphs.add(graph));
                    door_state.animation_index = Some(animation_index);
                    animation_player.play(animation_index);
                }

                debug!("playing open animation with speed {speed}");
                animation_player.adjust_speeds(speed);
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
                debug!("removing path of deleted actor `{entity}`");
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
