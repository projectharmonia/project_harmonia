use bevy::prelude::*;
use oxidized_navigation::{NavMesh, NavMeshSettings};

use crate::core::game_world::WorldName;

use super::{ComputePath, NavPath};

pub(super) struct EndpointPlugin;

impl Plugin for EndpointPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (Self::init_system, Self::cleanup_system).run_if(resource_exists::<WorldName>),
        );
    }
}

impl EndpointPlugin {
    fn init_system(
        mut commands: Commands,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        actors: Query<(Entity, &Transform, &Endpoint), Added<Endpoint>>,
    ) {
        for (entity, transform, endpoint) in &actors {
            commands.entity(entity).insert(ComputePath::new(
                nav_mesh.get(),
                nav_settings.clone(),
                transform.translation,
                endpoint.0,
            ));
        }
    }

    fn cleanup_system(mut commands: Commands, actors: Query<(Entity, &NavPath), Changed<NavPath>>) {
        for (entity, nav_path) in &actors {
            if nav_path.is_empty() {
                if let Some(mut commands) = commands.get_entity(entity) {
                    commands.remove::<Endpoint>();
                }
            }
        }
    }
}

/// Computes [`NavPath`] once after insertion.
#[derive(Component)]
pub(crate) struct Endpoint(Vec3);

impl Endpoint {
    pub(crate) fn new(point: Vec3) -> Self {
        Self(point)
    }
}
