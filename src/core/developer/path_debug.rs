use bevy::prelude::*;
use bevy_polyline::prelude::*;

use crate::core::{
    navigation::NavPath,
    settings::{Settings, SettingsApplySet},
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct PathDebugSet;

pub(super) struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathDebugMaterial>()
            .configure_set(PathDebugSet.run_if(debug_paths_enabled))
            .add_systems((Self::init_system, Self::despawn_system).in_set(PathDebugSet))
            .add_system(
                Self::cleanup_system
                    .in_set(SettingsApplySet)
                    .run_if(not(debug_paths_enabled)),
            );
    }
}

impl PathDebugPlugin {
    fn init_system(
        mut commands: Commands,
        mut polylines: ResMut<Assets<Polyline>>,
        path_material: Res<PathDebugMaterial>,
        actors: Query<(Entity, &Parent, &Transform, &NavPath), Added<NavPath>>,
    ) {
        for (entity, parent, transform, nav_path) in &actors {
            commands.entity(parent.get()).with_children(|parent| {
                let mut vertices = nav_path.0.clone();
                vertices.push(transform.translation);
                parent.spawn(PathDebugBundle::new(
                    entity,
                    path_material.0.clone(),
                    polylines.add(Polyline { vertices }),
                ));
            });
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut removed_paths: RemovedComponents<NavPath>,
        debug_paths: Query<(Entity, &NavActor)>,
    ) {
        for actor_entity in &mut removed_paths {
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

fn debug_paths_enabled(settings: Res<Settings>) -> bool {
    settings.developer.debug_paths
}

/// Stores a handle for the navigation debug line material.
#[derive(Resource)]
struct PathDebugMaterial(Handle<PolylineMaterial>);

impl FromWorld for PathDebugMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut polyline_materials = world.resource_mut::<Assets<PolylineMaterial>>();
        let material_handle = polyline_materials.add(PolylineMaterial {
            color: Color::INDIGO,
            perspective: true,
            ..Default::default()
        });
        Self(material_handle)
    }
}

#[derive(Bundle)]
struct PathDebugBundle {
    name: Name,
    nav_actor: NavActor,

    #[bundle]
    polyline_bundle: PolylineBundle,
}

impl PathDebugBundle {
    fn new(
        actor_entity: Entity,
        material_handle: Handle<PolylineMaterial>,
        polyline_handle: Handle<Polyline>,
    ) -> Self {
        Self {
            name: "Navigation polyline".into(),
            nav_actor: NavActor(actor_entity),
            polyline_bundle: PolylineBundle {
                polyline: polyline_handle,
                material: material_handle,
                ..Default::default()
            },
        }
    }
}

/// Stores entity to the associated moving actor.
///
/// Used for cleanup.
#[derive(Component)]
struct NavActor(Entity);
