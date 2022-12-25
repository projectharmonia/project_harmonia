pub(crate) mod editing_lot;
pub(crate) mod moving_lot;

use bevy::prelude::*;
use bevy_polyline::prelude::*;
use bevy_renet::renet::RenetServer;
use derive_more::Display;
use itertools::Itertools;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tap::TapFallible;

use super::{
    game_world::{GameEntity, GameWorld},
    network::network_event::{
        client_event::{ClientEvent, ClientEventAppExt},
        server_event::{SendMode, ServerEvent, ServerEventAppExt},
    },
};
use editing_lot::EditingLotPlugin;
use moving_lot::MovingLotPlugin;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter)]
pub(crate) enum LotTool {
    Edit,
    Move,
}

impl LotTool {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Edit => "✏",
            Self::Move => "↔",
        }
    }
}

pub(super) struct LotPlugin;

impl Plugin for LotPlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(LotTool::Edit)
            .add_plugin(EditingLotPlugin)
            .add_plugin(MovingLotPlugin)
            .register_type::<Vec<Vec2>>()
            .register_type::<LotVertices>()
            .add_client_event::<LotSpawn>()
            .add_client_event::<LotMove>()
            .add_server_event::<LotEventConfirmed>()
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::vertices_update_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::spawn_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::movement_system.run_if_resource_exists::<RenetServer>());
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
        mut confirm_events: EventWriter<ServerEvent<LotEventConfirmed>>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.iter().cloned() {
            commands.entity(event.city_entity).with_children(|parent| {
                parent.spawn((LotVertices(event.vertices), GameEntity));
            });
            confirm_events.send(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
        }
    }

    fn movement_system(
        mut move_events: EventReader<ClientEvent<LotMove>>,
        mut confirm_events: EventWriter<ServerEvent<LotEventConfirmed>>,
        mut lots: Query<&mut LotVertices>,
    ) {
        for ClientEvent { client_id, event } in move_events.iter().copied() {
            if let Ok(mut vertices) = lots
                .get_mut(event.entity)
                .tap_err(|e| error!("unable to apply lot movement from client {client_id}: {e}"))
            {
                for vertex in &mut vertices.0 {
                    *vertex += event.offset;
                }
                confirm_events.send(ServerEvent {
                    mode: SendMode::Direct(client_id),
                    event: LotEventConfirmed,
                });
            }
        }
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(super) struct LotVertices(Vec<Vec2>);

impl LotVertices {
    /// Converts polygon points to 3D coordinates with y = 0.
    #[must_use]
    fn to_plain(&self) -> Vec<Vec3> {
        self.iter()
            .map(|point| Vec3::new(point.x, 0.0, point.y))
            .collect()
    }

    /// A port of W. Randolph Franklin's [PNPOLY](https://wrf.ecse.rpi.edu//Research/Short_Notes/pnpoly.html) algorithm.
    #[must_use]
    fn contains_point(&self, point: Vec2) -> bool {
        let mut inside = false;
        for (a, b) in self.iter().tuple_windows() {
            if ((a.y > point.y) != (b.y > point.y))
                && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x)
            {
                inside = !inside;
            }
        }

        inside
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct LotMove {
    entity: Entity,
    offset: Vec2,
}

#[derive(Debug, Deserialize, Serialize)]
struct LotEventConfirmed;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_point() {
        let vertices = LotVertices(vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 1.0),
        ]);
        assert!(vertices.contains_point(Vec2::new(1.2, 1.9)));
    }

    #[test]
    fn not_contains_point() {
        let vertices = LotVertices(vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 1.0),
        ]);
        assert!(!vertices.contains_point(Vec2::new(3.2, 4.9)));
    }
}
