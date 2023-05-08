use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use oxidized_navigation::{NavMesh, NavMeshSettings};

use super::{ComputePath, Navigation};
use crate::core::game_world::WorldState;

pub(super) struct FollowPlugin;

impl Plugin for FollowPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                Self::following_system.run_if(on_timer(Duration::from_secs(1))),
                Self::cleanup_system,
            )
                .in_set(OnUpdate(WorldState::InWorld)),
        );
    }
}

impl FollowPlugin {
    fn following_system(
        mut commands: Commands,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        targets: Query<(&Transform, &FollowTarget), Changed<Transform>>,
        transforms: Query<&Transform>,
    ) {
        for (target_transform, followed) in &targets {
            let transform = transforms
                .get(followed.0)
                .expect("following entity should have transform");
            commands.entity(followed.0).insert(ComputePath::new(
                nav_mesh.get(),
                nav_settings.clone(),
                transform.translation,
                target_transform.translation,
            ));
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut removed_navigations: RemovedComponents<Navigation>,
        targets: Query<(Entity, &FollowTarget)>,
    ) {
        for following_entity in &mut removed_navigations {
            if let Some((target_entity, _)) = targets
                .iter()
                .find(|(_, followed)| followed.0 == following_entity)
            {
                commands.entity(target_entity).remove::<FollowTarget>();
            }
        }
    }
}

/// Updates the navigation path of the specified entity if an entity
/// containing this component changes its transform.
#[derive(Component)]
struct FollowTarget(Entity);
