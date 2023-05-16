pub(super) mod endpoint;
pub(super) mod following;

use std::sync::{Arc, RwLock};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use derive_more::Constructor;
use futures_lite::future;
use oxidized_navigation::{query, tiles::NavMeshTiles, NavMeshSettings};

use crate::core::game_world::WorldState;
use endpoint::EndpointPlugin;
use following::FollowingPlugin;

pub(super) struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EndpointPlugin)
            .add_plugin(FollowingPlugin)
            .add_systems(
                (
                    Self::navigation_system,
                    Self::poll_system,
                    Self::cleanup_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
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

                if direction.length() < 0.1 {
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
        for entity in &mut removed_navigations {
            if let Some(mut commands) = commands.get_entity(entity) {
                commands.remove::<(NavPath, ComputePath)>();
            }
        }
    }
}

#[derive(Component, Constructor)]
pub(super) struct Navigation {
    speed: f32,
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
            let path = query::find_path(&tiles, &settings, start, end, None, None)
                .expect("navigation should happen only inside the city");

            query::perform_string_pulling_on_path(&tiles, start, end, &path)
                .expect("passed tiles should be valid and connected")
        });

        Self(task)
    }
}

#[derive(Component, Deref, DerefMut)]
pub(super) struct NavPath(pub(super) Vec<Vec3>);
