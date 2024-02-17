pub(super) mod endpoint;
pub(super) mod following;

use std::sync::{Arc, RwLock};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use oxidized_navigation::{query, tiles::NavMeshTiles, NavMeshSettings};

use super::game_world::WorldName;
use endpoint::EndpointPlugin;
use following::FollowingPlugin;

pub(super) struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((EndpointPlugin, FollowingPlugin))
            .add_systems(
                PreUpdate,
                Self::poll_system.run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                Update,
                Self::navigation_system.run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup_system.run_if(resource_exists::<WorldName>()),
            );
    }
}

impl NavigationPlugin {
    fn poll_system(mut commands: Commands, mut actors: Query<(Entity, &mut ComputePath)>) {
        for (entity, mut compute_path) in &mut actors {
            if let Some(mut path) = future::block_on(future::poll_once(&mut compute_path.0)) {
                path.reverse();
                path.pop(); // Drop current position.
                commands
                    .entity(entity)
                    .insert(NavPath(path))
                    .remove::<ComputePath>();
            }
        }
    }

    fn navigation_system(
        mut commands: Commands,
        time: Res<Time>,
        mut actors: Query<(Entity, &Navigation, &mut Transform, &mut NavPath)>,
    ) {
        for (entity, navigation, mut transform, mut nav_path) in &mut actors {
            if let Some(&waypoint) = nav_path.last() {
                const ROTATION_SPEED: f32 = 10.0;
                let direction = waypoint - transform.translation;
                let delta_secs = time.delta_seconds();
                let target_rotation = transform.looking_to(direction, Vec3::Y).rotation;

                transform.translation += direction.normalize() * navigation.speed * delta_secs;
                transform.rotation = transform
                    .rotation
                    .slerp(target_rotation, ROTATION_SPEED * delta_secs);

                let min_distance = if nav_path.len() == 1 {
                    // Last waypoint.
                    navigation.offset.max(DISTANCE_EPSILON)
                } else {
                    DISTANCE_EPSILON
                };
                if direction.length() < min_distance {
                    nav_path.pop();
                }
            } else {
                commands.entity(entity).remove::<Navigation>();
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
    ) {
        for entity in removed_navigations.read() {
            if let Some(mut commands) = commands.get_entity(entity) {
                commands.remove::<(NavPath, ComputePath)>();
            }
        }
    }
}

const DISTANCE_EPSILON: f32 = 0.1;

#[derive(Component)]
pub(super) struct Navigation {
    pub(super) speed: f32,
    /// Offset for the last waypoint.
    pub(super) offset: f32,
}

impl Navigation {
    pub(super) fn new(speed: f32) -> Self {
        Self {
            speed,
            offset: DISTANCE_EPSILON,
        }
    }

    pub(super) fn with_offset(mut self, offset: f32) -> Self {
        self.offset = offset;
        self
    }
}

#[derive(Component)]
struct ComputePath(Task<Vec<Vec3>>);

impl ComputePath {
    fn new(
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

#[derive(Component, Deref, DerefMut)]
pub(super) struct NavPath(pub(super) Vec<Vec3>);
