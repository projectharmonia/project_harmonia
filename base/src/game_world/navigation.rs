pub(super) mod following;
pub(super) mod path_debug;

use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};
use bevy_replicon::prelude::*;
use oxidized_navigation::{
    query::{self, FindPathError},
    tiles::NavMeshTiles,
    NavMesh, NavMeshSettings,
};
use path_debug::PathDebugPlugin;
use serde::{Deserialize, Serialize};

use following::FollowingPlugin;

pub(super) struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FollowingPlugin, PathDebugPlugin))
            .register_type::<NavSettings>()
            .register_type::<NavDestination>()
            .replicate::<NavSettings>()
            .replicate::<NavDestination>()
            .replicate::<NavPath>()
            .add_systems(
                PreUpdate,
                Self::generate_paths
                    .after(ClientSet::Receive)
                    .run_if(server_or_singleplayer),
            )
            .add_systems(Update, Self::navigate.run_if(server_or_singleplayer));
    }
}

impl NavigationPlugin {
    // TODO: Regenerate paths when tiles update: https://github.com/TheGrimsey/oxidized_navigation/issues/31
    fn generate_paths(
        nav_mesh_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        cities: Query<&GlobalTransform>,
        mut agents: Query<(
            Entity,
            &Parent,
            &Transform,
            &mut NavDestination,
            &mut NavPath,
            &mut WaypointIndex,
        )>,
    ) {
        for (entity, parent, transform, mut dest, mut path, mut waypoint_index) in &mut agents {
            if dest.is_changed() {
                debug!("resetting old path for `{entity}`");
                path.0.clear();
                waypoint_index.0 = 0;
            }

            let Some(endpoint) = **dest else {
                continue;
            };

            if !path.is_empty() {
                // The path has already been generated and
                // the destination has not been changed.
                continue;
            }

            let tiles = nav_mesh.get();
            let Ok(tiles) = tiles.read() else {
                continue;
            };

            let city_transform = *cities.get(**parent).unwrap();

            match transformed_path(
                &tiles,
                &nav_mesh_settings,
                city_transform,
                transform.translation,
                endpoint,
            ) {
                Ok(new_path) => {
                    debug!("updating path for `{entity}`");
                    path.0 = new_path
                }
                Err(FindPathError::PolygonPath(e)) => {
                    // A tile or mesh is not generated yet.
                    trace!("delaying pathfinding for `{entity}` due to `{e:?}`")
                }
                Err(FindPathError::StringPulling(e)) => {
                    debug!("denying destination for `{entity}` due to `{e:?}`");
                    **dest = None;
                }
            }
        }
    }

    fn navigate(
        time: Res<Time>,
        mut agents: Query<(
            Entity,
            &NavSettings,
            &NavPath,
            &mut WaypointIndex,
            &mut NavDestination,
            &mut Transform,
        )>,
    ) {
        for (entity, nav_settings, path, mut waypoint_index, mut dest, mut transform) in &mut agents
        {
            if dest.is_none() || path.is_empty() {
                continue;
            }

            let next_index = **waypoint_index + 1;
            let next_waypoint = path[next_index];
            let disp = next_waypoint - transform.translation;
            let delta_secs = time.delta_seconds();
            let target_rotation = transform.looking_to(disp, Vec3::Y).rotation;

            const ROTATION_SPEED: f32 = 10.0;
            transform.translation += disp.normalize() * nav_settings.speed * delta_secs;
            transform.rotation = transform
                .rotation
                .slerp(target_rotation, ROTATION_SPEED * delta_secs);

            const DISTANCE_EPSILON: f32 = 0.1;
            if next_index == path.len() - 1 {
                if disp.length() < nav_settings.offset.unwrap_or(DISTANCE_EPSILON) {
                    debug!("`{entity}` finished navigation");
                    **dest = None;
                }
            } else if disp.length() < DISTANCE_EPSILON {
                waypoint_index.0 += 1;
                debug!(
                    "advancing waypoint index to {}/{} for `{entity}`",
                    **waypoint_index,
                    path.len(),
                );
            }
        }
    }
}

fn transformed_path(
    tiles: &NavMeshTiles,
    nav_mesh_settings: &NavMeshSettings,
    city_transform: GlobalTransform,
    start: Vec3,
    end: Vec3,
) -> Result<Vec<Vec3>, FindPathError> {
    let mut path = query::find_path(
        tiles,
        nav_mesh_settings,
        city_transform.transform_point(start),
        city_transform.transform_point(end),
        None,
        None,
    )?;
    let inversed_affine = city_transform.affine().inverse();
    for point in &mut path {
        *point = inversed_affine.transform_vector3(*point);
    }

    Ok(path)
}

#[derive(Bundle, Default)]
pub(super) struct NavigationBundle {
    nav_settings: NavSettings,
    dest: NavDestination,
    path: NavPath,
}

/// Navigation parameters.
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(super) struct NavSettings {
    /// Movement speed.
    speed: f32,

    /// Offset for the last waypoint.
    offset: Option<f32>,
}

impl NavSettings {
    pub(super) fn new(speed: f32) -> Self {
        Self {
            speed,
            offset: None,
        }
    }

    pub(super) fn with_offset(mut self, offset: f32) -> Self {
        self.offset = Some(offset);
        self
    }

    pub(super) fn speed(&self) -> f32 {
        self.speed
    }
}

/// Defines navigation destination point.
///
/// Changing this component to [`Some`] will trigger [`NavPath`] calculation.
/// Calculation could take more then one frame if navigation mesh updates and until then [`NavPath`] will be cleared.
/// If set to [`None`], just clears [`NavPath`].
#[derive(Reflect, Deref, DerefMut, Default, Serialize, Deserialize)]
#[reflect(Component)]
pub(super) struct NavDestination(Option<Vec3>);

// TODO 0.15: Replace with required components.
impl Component for NavDestination {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, targeted_entity, _component_id| {
            if world.get::<WaypointIndex>(targeted_entity).is_none() {
                world
                    .commands()
                    .entity(targeted_entity)
                    .insert(WaypointIndex::default());
            }
            if world.get::<NavPath>(targeted_entity).is_none() {
                world
                    .commands()
                    .entity(targeted_entity)
                    .insert(NavPath::default());
            }
        });
    }
}

/// Calculated navigation path.
///
/// Includes start point, itermediate points and the destination point.
/// This component updates each time [`NavDestination`] changes.
#[derive(Default, Deref, Component, Serialize, Deserialize)]
pub(super) struct NavPath(Vec<Vec3>);

/// Index of the last reached waypoint from [`NavPath`].
///
/// Updated each time agent reaches a waypoint from it's path.
/// Resets to 0 each time [`NavPath`] changes.
#[derive(Component, Default, Serialize, Deserialize, Deref)]
struct WaypointIndex(usize);
