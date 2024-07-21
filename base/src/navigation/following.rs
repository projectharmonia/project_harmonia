use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_replicon::prelude::*;
use oxidized_navigation::{NavMesh, NavMeshSettings};

use super::{ComputePath, NavPath};

pub(super) struct FollowingPlugin;

impl Plugin for FollowingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // Should run in `Update` to let tiles initialize.
                Self::init,
                Self::update_path.run_if(on_timer(Duration::from_secs(1))),
            )
                .run_if(has_authority),
        )
        .add_systems(PostUpdate, Self::stop_following.run_if(has_authority));
    }
}

impl FollowingPlugin {
    fn init(
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

            debug!("setting path to target `{entity}`");
            commands.entity(entity).insert(ComputePath::new(
                nav_mesh.get(),
                nav_settings.clone(),
                transform.translation,
                target_transform.translation,
            ));
        }
    }

    fn update_path(
        mut commands: Commands,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        followers: Query<(Entity, &Transform, &Following)>,
        transforms: Query<&Transform, Changed<Transform>>,
    ) {
        for (entity, transform, following) in &followers {
            if let Ok(target_transform) = transforms.get(following.0) {
                debug!("updating path to target `{entity}`");
                commands.entity(entity).insert(ComputePath::new(
                    nav_mesh.get(),
                    nav_settings.clone(),
                    transform.translation,
                    target_transform.translation,
                ));
            }
        }
    }

    fn stop_following(
        mut commands: Commands,
        followers: Query<(Entity, &NavPath), (Changed<NavPath>, With<Following>)>,
    ) {
        for (entity, nav_path) in &followers {
            if nav_path.is_empty() {
                if let Some(mut commands) = commands.get_entity(entity) {
                    commands.remove::<Following>();
                }
            }
        }
    }
}

/// Updates the navigation path if the specified entity changes its transform.
#[derive(Component)]
pub(crate) struct Following(pub(crate) Entity);
