pub(super) mod following;
pub(super) mod path_debug;

use std::sync::{Arc, RwLock};

use bevy::{
    prelude::*,
    tasks::{futures_lite::future, AsyncComputeTaskPool, Task},
};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use oxidized_navigation::{query, tiles::NavMeshTiles, NavMeshSettings};
use path_debug::PathDebugPlugin;
use serde::{Deserialize, Serialize};

use super::math;
use following::FollowingPlugin;

pub(super) struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FollowingPlugin, PathDebugPlugin))
            .register_type::<Navigation>()
            .register_type::<NavPath>()
            .register_type::<WaypointIndex>()
            .replicate::<Navigation>()
            .replicate::<NavPath>()
            .replicate::<WaypointIndex>()
            .replicate::<Position>()
            .replicate::<Rotation>()
            .add_systems(PreUpdate, Self::poll_paths.run_if(has_authority))
            .add_systems(Update, Self::navigate.run_if(has_authority));
    }
}

impl NavigationPlugin {
    fn poll_paths(
        mut commands: Commands,
        mut actors: Query<(Entity, &mut ComputePath, &mut NavPath)>,
    ) {
        for (entity, mut compute_path, mut nav_path) in &mut actors {
            if let Some(path) = future::block_on(future::poll_once(&mut compute_path.0)) {
                debug!("computed path for `{entity}`");
                nav_path.0 = path;
                commands.entity(entity).remove::<ComputePath>();
            }
        }
    }

    fn navigate(
        time: Res<Time>,
        mut actors: Query<(
            Entity,
            &Navigation,
            &mut NavPath,
            &mut WaypointIndex,
            &mut Position,
            &mut Rotation,
        )>,
    ) {
        for (entity, navigation, mut nav_path, mut waypoint_index, mut position, mut rotation) in
            &mut actors
        {
            if nav_path.len() <= 1 {
                continue;
            }

            // Reset current waypoint index when navigation path changes.
            if nav_path.is_changed() {
                debug!("resetting waypoint index for `{entity}`");
                waypoint_index.0 = 1; // Always skip first waypoint since it's initial position.
            }

            let waypoint = nav_path.0[waypoint_index.0];
            let disp = waypoint - **position;
            let delta_secs = time.delta_seconds();
            let target_rotation = math::looking_to(disp);

            const ROTATION_SPEED: f32 = 10.0;
            **position += disp.normalize() * navigation.speed * delta_secs;
            **rotation = rotation.slerp(target_rotation, ROTATION_SPEED * delta_secs);

            const DISTANCE_EPSILON: f32 = 0.1;
            if waypoint_index.0 == nav_path.len() - 1 {
                if disp.length() < navigation.offset.unwrap_or(DISTANCE_EPSILON) {
                    debug!("`{entity}` finished navigation");
                    nav_path.clear();
                }
            } else if disp.length() < DISTANCE_EPSILON {
                waypoint_index.0 += 1;
                debug!(
                    "incremented waypoint index to {} for `{entity}`",
                    waypoint_index.0
                );
            }
        }
    }
}

#[derive(Bundle, Default, Reflect)]
pub(super) struct NavigationBundle {
    navigation: Navigation,
    nav_path: NavPath,
    nav_point_index: WaypointIndex,
}

/// Navigation parameters.
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(super) struct Navigation {
    speed: f32,
    /// Offset for the last waypoint.
    offset: Option<f32>,
}

impl Navigation {
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

#[derive(Component)]
pub(super) struct ComputePath(Task<Vec<Vec3>>);

impl ComputePath {
    pub(super) fn new(
        tiles: Arc<RwLock<NavMeshTiles>>,
        settings: NavMeshSettings,
        start: Vec3,
        end: Vec3,
    ) -> Self {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(async move {
            let tiles = tiles.read().expect("tiles shouldn't be poisoned");
            query::find_path(&tiles, &settings, start, end, None, None)
                .expect("navigation should happen only inside the city")
        });

        Self(task)
    }
}

/// Stores navigation path.
#[derive(Component, Default, Deref, DerefMut, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(super) struct NavPath(pub(super) Vec<Vec3>);

/// Index of the current waypoint from [`NavPath`].
#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(super) struct WaypointIndex(usize);
