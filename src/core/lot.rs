pub(crate) mod editing_lot;

use bevy::prelude::*;
use bevy_polyline::prelude::*;
use bevy_renet::renet::RenetServer;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    game_world::GameWorld,
    network::network_event::{
        client_event::{ClientEvent, ClientEventAppExt},
        server_event::{SendMode, ServerEvent, ServerEventAppExt},
    },
};
use editing_lot::EditingLotPlugin;

pub(super) struct LotPlugin;

impl Plugin for LotPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(EditingLotPlugin)
            .add_client_event::<LotSpawn>()
            .add_server_event::<LotSpawnConfirmed>()
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::vertices_update_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::spawn_system.run_if_resource_exists::<RenetServer>());
    }
}

impl LotPlugin {
    fn init_system(
        lot_material: Local<LotMaterial>,
        mut commands: Commands,
        mut polylines: ResMut<Assets<Polyline>>,
        spawned_lots: Query<(Entity, &LotVertices), Added<LotVertices>>,
    ) {
        for (entity, vertices) in &spawned_lots {
            commands.entity(entity).insert(PolylineBundle {
                polyline: polylines.add(Polyline {
                    vertices: vertices.to_plain(),
                }),
                material: lot_material.0.clone(),
                ..Default::default()
            });
        }
    }

    fn vertices_update_system(
        mut polylines: ResMut<Assets<Polyline>>,
        changed_lots: Query<(&Handle<Polyline>, &LotVertices, ChangeTrackers<LotVertices>)>,
    ) {
        for (polyline_handle, vertices, changed_vertices) in &changed_lots {
            if changed_vertices.is_changed() && !changed_vertices.is_added() {
                let polyline = polylines
                    .get_mut(polyline_handle)
                    .expect("polyline should be spawned on init");
                polyline.vertices = vertices.to_plain();
            }
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: EventReader<ClientEvent<LotSpawn>>,
        mut confirm_events: EventWriter<ServerEvent<LotSpawnConfirmed>>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.iter().cloned() {
            commands.entity(event.city_entity).with_children(|parent| {
                parent.spawn(LotVertices(event.vertices));
            });
            confirm_events.send(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: LotSpawnConfirmed,
            });
        }
    }
}

#[derive(Component, Deref, DerefMut)]
struct LotVertices(Vec<Vec2>);

impl LotVertices {
    /// Converts polygon points to 3D coordinates with y = 0.
    fn to_plain(&self) -> Vec<Vec3> {
        self.iter()
            .map(|point| Vec3::new(point.x, 0.0, point.y))
            .collect()
    }
}

/// Stores a handle for the lot line material.
#[derive(Resource)]
struct LotMaterial(Handle<PolylineMaterial>);

impl FromWorld for LotMaterial {
    fn from_world(world: &mut World) -> Self {
        let mut polyline_materials = world.resource_mut::<Assets<PolylineMaterial>>();
        let material_handle = polyline_materials.add(PolylineMaterial {
            color: Color::WHITE,
            perspective: true,
            ..Default::default()
        });
        Self(material_handle)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct LotSpawn {
    vertices: Vec<Vec2>,
    city_entity: Entity,
}

#[derive(Debug, Deserialize, Serialize)]
struct LotSpawnConfirmed;
