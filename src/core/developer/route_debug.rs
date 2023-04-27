use bevy::prelude::*;
use bevy_polyline::prelude::*;

use crate::core::{
    actor::movement::MovePath,
    settings::{Settings, SettingsApplySet},
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct RouteDebugSet;

pub(super) struct RouteDebugPlugin;

impl Plugin for RouteDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RouteDebugMaterial>()
            .configure_set(RouteDebugSet.run_if(debug_routes_enabled))
            .add_systems((Self::init_system, Self::despawn_system).in_set(RouteDebugSet))
            .add_system(
                Self::cleanup_system
                    .in_set(SettingsApplySet)
                    .run_if(not(debug_routes_enabled)),
            );
    }
}

impl RouteDebugPlugin {
    fn init_system(
        mut commands: Commands,
        mut polylines: ResMut<Assets<Polyline>>,
        route_material: Res<RouteDebugMaterial>,
        actors: Query<(Entity, &Parent, &Transform, &MovePath), Added<MovePath>>,
    ) {
        for (entity, parent, transform, move_path) in &actors {
            commands.entity(parent.get()).with_children(|parent| {
                let mut vertices = move_path.0.clone();
                vertices.push(transform.translation);
                parent.spawn(RouteDebugBundle::new(
                    entity,
                    route_material.0.clone(),
                    polylines.add(Polyline { vertices }),
                ));
            });
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut removed_paths: RemovedComponents<MovePath>,
        routes: Query<(Entity, &MoveActor)>,
    ) {
        for actor_entity in &mut removed_paths {
            if let Some((route_entity, _)) = routes
                .iter()
                .find(|(_, move_actor)| move_actor.0 == actor_entity)
            {
                commands.entity(route_entity).despawn();
            }
        }
    }

    fn cleanup_system(mut commands: Commands, routes: Query<Entity, With<MoveActor>>) {
        for entity in &routes {
            commands.entity(entity).despawn();
        }
    }
}

fn debug_routes_enabled(settings: Res<Settings>) -> bool {
    settings.developer.debug_routes
}

/// Stores a handle for the navigation debug line material.
#[derive(Resource)]
struct RouteDebugMaterial(Handle<PolylineMaterial>);

impl FromWorld for RouteDebugMaterial {
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
struct RouteDebugBundle {
    name: Name,
    move_actor: MoveActor,

    #[bundle]
    polyline_bundle: PolylineBundle,
}

impl RouteDebugBundle {
    fn new(
        actor_entity: Entity,
        material_handle: Handle<PolylineMaterial>,
        polyline_handle: Handle<Polyline>,
    ) -> Self {
        Self {
            name: "Navigation polyline".into(),
            move_actor: MoveActor(actor_entity),
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
struct MoveActor(Entity);
