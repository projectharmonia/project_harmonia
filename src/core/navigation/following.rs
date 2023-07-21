use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use oxidized_navigation::{NavMesh, NavMeshSettings};

use super::{ComputePath, Navigation};
use crate::core::game_world::WorldName;

pub(super) struct FollowingPlugin;

impl Plugin for FollowingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::init_system,
                Self::following_system.run_if(on_timer(Duration::from_secs(1))),
                Self::cleanup_system,
            )
                .run_if(resource_exists::<WorldName>()),
        );
    }
}

impl FollowingPlugin {
    fn init_system(
        mut commands: Commands,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        followers: Query<(Entity, &Transform, &Following), Changed<Following>>,
        transforms: Query<&Transform>,
    ) {
        for (entity, transform, following) in &followers {
            let target_transform = transforms
                .get(following.0)
                .expect("target entity should have transform");

            commands.entity(entity).insert(ComputePath::new(
                nav_mesh.get(),
                nav_settings.clone(),
                transform.translation,
                target_transform.translation,
            ));
        }
    }

    fn following_system(
        mut commands: Commands,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        followers: Query<(Entity, &Transform, &Following)>,
        transforms: Query<&Transform, Changed<Transform>>,
    ) {
        for (entity, transform, following) in &followers {
            if let Ok(target_transform) = transforms.get(following.0) {
                commands.entity(entity).insert(ComputePath::new(
                    nav_mesh.get(),
                    nav_settings.clone(),
                    transform.translation,
                    target_transform.translation,
                ));
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
    ) {
        for entity in &mut removed_navigations {
            if let Some(mut commands) = commands.get_entity(entity) {
                commands.remove::<Following>();
            }
        }
    }
}

/// Updates the navigation path if the specified entity changes its transform.
#[derive(Component)]
pub(crate) struct Following(pub(crate) Entity);
