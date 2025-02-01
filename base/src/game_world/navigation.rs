pub(super) mod following;
pub(super) mod path_debug;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use path_debug::PathDebugPlugin;
use serde::{Deserialize, Serialize};
use vleue_navigator::prelude::*;

use crate::game_world::city::CityNavMesh;
use following::FollowingPlugin;

pub(super) struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((FollowingPlugin, PathDebugPlugin))
            .register_type::<Navigation>()
            .register_type::<NavDestination>()
            .replicate::<Navigation>()
            .replicate::<NavDestination>()
            .replicate::<NavPath>()
            .add_systems(
                PreUpdate,
                (update_paths, generate_paths)
                    .chain()
                    .after(ClientSet::Receive)
                    .run_if(server_or_singleplayer),
            )
            .add_systems(Update, navigate.run_if(server_or_singleplayer));
    }
}

/// Updates path on navmesh changes.
fn update_paths(
    mut navmeshes: ResMut<Assets<NavMesh>>,
    city_navmeshes: Query<(&ManagedNavMesh, &Parent, &NavMeshStatus), Changed<NavMeshStatus>>,
    children: Query<&Children>,
    mut agents: Query<(
        Entity,
        &Transform,
        &mut NavDestination,
        &mut NavPath,
        &mut NavPathIndex,
    )>,
) {
    for (navmesh_handle, parent, status) in &city_navmeshes {
        if !matches!(status, NavMeshStatus::Built) {
            continue;
        }

        let Some(navmesh) = navmeshes.get_mut(navmesh_handle) else {
            continue;
        };

        let children = children.get(**parent).unwrap();
        let mut iter = agents.iter_many_mut(children);
        while let Some((entity, transform, mut dest, mut path, mut path_index)) = iter.fetch_next()
        {
            let Some(endpoint) = **dest else {
                continue;
            };

            if let Some(transformed) = navmesh.transformed_path(transform.translation, endpoint) {
                debug!("recalculating path for `{entity}`");
                path.0.push(transform.translation);
                path.0.extend(transformed.path);
                path_index.0 = 0;
            } else {
                debug!("cancelling destination for `{entity}`");
                **dest = None;
            }
        }
    }
}

fn generate_paths(
    mut navmeshes: ResMut<Assets<NavMesh>>,
    cities: Query<&CityNavMesh>,
    city_navmeshes: Query<&ManagedNavMesh>,
    mut agents: Query<
        (
            Entity,
            &Parent,
            &Transform,
            &mut NavDestination,
            &mut NavPath,
            &mut NavPathIndex,
        ),
        Changed<NavDestination>,
    >,
) {
    for (entity, parent, transform, mut dest, mut path, mut path_index) in &mut agents {
        path.0.clear();
        path_index.0 = 0;

        let Some(endpoint) = **dest else {
            continue;
        };

        let navmesh_entity = cities
            .get(**parent)
            .expect("all agents should have city as parents");
        let navmesh_handle = city_navmeshes
            .get(**navmesh_entity)
            .expect("city navmesh should always be valid");

        let Some(navmesh) = navmeshes.get_mut(navmesh_handle) else {
            continue;
        };

        if let Some(transformed) = navmesh.transformed_path(transform.translation, endpoint) {
            debug!("calculating path for `{entity}`");
            path.0.push(transform.translation);
            path.0.extend(transformed.path);
        } else {
            debug!("refusing destination for `{entity}`");
            **dest = None;
        }
    }
}

fn navigate(
    time: Res<Time>,
    mut agents: Query<(
        Entity,
        &Navigation,
        &NavPath,
        &mut NavPathIndex,
        &mut NavDestination,
        &mut Transform,
    )>,
) {
    for (entity, &navigation, path, mut path_index, mut dest, mut transform) in &mut agents {
        if dest.is_none() || path.is_empty() {
            continue;
        }

        let target_index = **path_index + 1;
        if let Some(passed_points) = move_agent(
            &mut transform,
            navigation,
            &path[target_index..],
            time.delta_secs(),
        ) {
            if passed_points != 0 {
                **path_index += passed_points;
                debug!(
                    "advancing path index to {}/{} for `{entity}`",
                    **path_index,
                    path.len() - 1,
                );
            }
        } else {
            debug!("`{entity}` finished navigation");
            **dest = None;
        }
    }
}

/// Marks an entity with [`Collider`] as a navigation mesh affector.
#[derive(Component, Default)]
pub struct Obstacle;

/// Moves the agent along a path.
///
/// The path should contain only the remaining points to reach.
///
/// Skips points that actor have projected past to prevent jitter
/// when multiple points are near each other.
///
/// Returns the number of points passed.
/// If the path is completed, returns [`None`].
fn move_agent(
    transform: &mut Transform,
    navigation: Navigation,
    path: &[Vec3],
    delta: f32,
) -> Option<usize> {
    let movement_step = navigation.speed * delta;
    let (passed_points, &target_point) = path.iter().enumerate().find(|&(index, &point)| {
        const EPSILON: f32 = 0.1;
        let tolerance = if index == path.len() - 1 {
            // Apply the desired offset for the last point.
            navigation.offset.unwrap_or(EPSILON)
        } else {
            EPSILON
        };

        transform.translation.distance(point) - movement_step > tolerance
    })?;

    let disp = target_point - transform.translation;
    let target_rotation = transform.looking_to(disp, Vec3::Y).rotation;
    const ROTATION_SPEED: f32 = 10.0;
    transform.translation += disp.normalize() * movement_step;
    transform.rotation = transform
        .rotation
        .slerp(target_rotation, ROTATION_SPEED * delta);

    Some(passed_points)
}

/// Navigation parameters.
#[derive(Component, Clone, Copy, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
#[require(NavDestination, NavPath)]
pub(super) struct Navigation {
    /// Movement speed.
    speed: f32,

    /// Offset for the target point.
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

/// Defines navigation destination point.
///
/// Changing this component to [`Some`] will trigger [`NavPath`] calculation.
/// Calculation could take more then one frame if navigation mesh updates and until then [`NavPath`] will be cleared.
/// If set to [`None`], just clears [`NavPath`].
#[derive(Component, Reflect, Deref, DerefMut, Default, Serialize, Deserialize)]
#[reflect(Component)]
#[require(NavPath, NavPathIndex)]
pub(super) struct NavDestination(Option<Vec3>);

/// Calculated navigation path.
///
/// Includes start point, intermediate points and the destination point.
/// This component updates each time [`NavDestination`] changes.
#[derive(Default, Deref, Component, Serialize, Deserialize)]
pub(super) struct NavPath(Vec<Vec3>);

/// Index of the last reached point from [`NavPath`].
///
/// Updated each time agent reaches a point from it's path.
/// Resets to 0 each time [`NavPath`] changes.
#[derive(Component, Default, Serialize, Deserialize, Deref, DerefMut)]
struct NavPathIndex(usize);
