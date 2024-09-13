use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_replicon::prelude::*;

use super::NavDestination;

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
                .run_if(server_or_singleplayer),
        )
        .add_systems(
            PostUpdate,
            Self::stop_following.run_if(server_or_singleplayer),
        );
    }
}

impl FollowingPlugin {
    fn init(
        mut followers: Query<(Entity, &Following, &mut NavDestination), Changed<Following>>,
        transforms: Query<&Transform>,
    ) {
        for (entity, following, mut dest) in &mut followers {
            let target_transform = transforms
                .get(following.0)
                .expect("target entity should have transform");

            debug!("setting path to target `{entity}`");
            **dest = Some(target_transform.translation);
        }
    }

    fn update_path(
        mut followers: Query<(Entity, &Following, &mut NavDestination)>,
        transforms: Query<&Transform, Changed<Transform>>,
    ) {
        for (entity, following, mut dest) in &mut followers {
            if let Ok(target_transform) = transforms.get(following.0) {
                debug!("updating path to target `{entity}`");
                **dest = Some(target_transform.translation);
            }
        }
    }

    fn stop_following(
        mut commands: Commands,
        followers: Query<(Entity, &NavDestination), (Changed<NavDestination>, With<Following>)>,
    ) {
        for (entity, dest) in &followers {
            if dest.is_none() {
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
