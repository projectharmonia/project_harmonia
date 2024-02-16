use bevy::prelude::*;
use oxidized_navigation::debug_draw::DrawPath;

use crate::core::{
    navigation::NavPath,
    settings::{Settings, SettingsApply},
};

pub(super) struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (Self::init_system, Self::despawn_system).run_if(debug_paths_enabled()),
        )
        .add_systems(
            PostUpdate,
            Self::cleanup_system
                .run_if(on_event::<SettingsApply>())
                .run_if(not(debug_paths_enabled())),
        );
    }
}

impl PathDebugPlugin {
    fn init_system(
        mut commands: Commands,
        actors: Query<(Entity, &Parent, &Transform, &NavPath), Added<NavPath>>,
    ) {
        for (entity, parent, transform, nav_path) in &actors {
            commands.entity(parent.get()).with_children(|parent| {
                let mut pulled_path = nav_path.0.clone();
                pulled_path.push(transform.translation);
                parent.spawn(PathDebugBundle::new(entity, pulled_path));
            });
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut removed_paths: RemovedComponents<NavPath>,
        debug_paths: Query<(Entity, &NavActor)>,
    ) {
        for actor_entity in removed_paths.read() {
            if let Some((debug_entity, _)) = debug_paths
                .iter()
                .find(|(_, nav_actor)| nav_actor.0 == actor_entity)
            {
                commands.entity(debug_entity).despawn();
            }
        }
    }

    fn cleanup_system(mut commands: Commands, routes: Query<Entity, With<NavActor>>) {
        for entity in &routes {
            commands.entity(entity).despawn();
        }
    }
}

fn debug_paths_enabled() -> impl FnMut(Res<Settings>) -> bool {
    |settings| settings.developer.debug_paths
}

#[derive(Bundle)]
struct PathDebugBundle {
    name: Name,
    nav_actor: NavActor,
    draw_path: DrawPath,
}

impl PathDebugBundle {
    fn new(actor_entity: Entity, pulled_path: Vec<Vec3>) -> Self {
        Self {
            name: "Navigation path".into(),
            nav_actor: NavActor(actor_entity),
            draw_path: DrawPath {
                timer: None,
                pulled_path,
                color: Color::LIME_GREEN,
            },
        }
    }
}

/// Stores entity to the associated moving actor.
///
/// Used for cleanup.
#[derive(Component)]
struct NavActor(Entity);
