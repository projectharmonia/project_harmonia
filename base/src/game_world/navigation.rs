pub(super) mod following;
pub(super) mod path_debug;

use avian3d::prelude::*;
use bevy::{
    ecs::component::{ComponentHooks, StorageType},
    prelude::*,
};
use bevy_replicon::prelude::*;
use path_debug::PathDebugPlugin;
use serde::{Deserialize, Serialize};
use vleue_navigator::prelude::*;

use crate::{game_world::city::CityNavMesh, math};
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
            .replicate::<Position>()
            .replicate::<Rotation>()
            .add_systems(
                PreUpdate,
                (Self::update_paths, Self::generate_paths)
                    .chain()
                    .after(ClientSet::Receive)
                    .run_if(server_or_singleplayer),
            )
            .add_systems(Update, Self::navigate.run_if(server_or_singleplayer));
    }
}

impl NavigationPlugin {
    /// Updates path on navmesh changes.
    fn update_paths(
        mut navmeshes: ResMut<Assets<NavMesh>>,
        city_navmeshes: Query<(&Handle<NavMesh>, &Parent, &NavMeshStatus), Changed<NavMeshStatus>>,
        children: Query<&Children>,
        mut agents: Query<(
            Entity,
            &Transform,
            &mut NavDestination,
            &mut NavPath,
            &mut WaypointIndex,
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
            while let Some((entity, transform, mut dest, mut path, mut waypoint_index)) =
                iter.fetch_next()
            {
                let Some(endpoint) = **dest else {
                    continue;
                };

                if let Some(transformed) = navmesh.transformed_path(transform.translation, endpoint)
                {
                    debug!("recalculating path for `{entity}`");
                    path.0.push(transform.translation);
                    path.0.extend(transformed.path);
                    waypoint_index.0 = 0;
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
        city_navmeshes: Query<&Handle<NavMesh>>,
        mut agents: Query<
            (
                Entity,
                &Parent,
                &Transform,
                &mut NavDestination,
                &mut NavPath,
                &mut WaypointIndex,
            ),
            Changed<NavDestination>,
        >,
    ) {
        for (entity, parent, transform, mut dest, mut path, mut waypoint_index) in &mut agents {
            path.0.clear();
            waypoint_index.0 = 0;

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
            let target_rotation = math::looking_to(disp);

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

/// Marks an entity with [`Collider`] as a navigation mesh affector.
#[derive(Component)]
pub struct Obstacle;

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
