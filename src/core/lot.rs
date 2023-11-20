pub(crate) mod creating_lot;
pub(crate) mod moving_lot;

use bevy::prelude::*;
use bevy_polyline::prelude::*;
use bevy_replicon::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::game_world::WorldName;
use creating_lot::CreatingLotPlugin;
use moving_lot::MovingLotPlugin;

pub(super) struct LotPlugin;

impl Plugin for LotPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<LotTool>()
            .add_plugins((CreatingLotPlugin, MovingLotPlugin))
            .register_type::<Vec<Vec2>>()
            .register_type::<LotVertices>()
            .replicate::<LotVertices>()
            .add_mapped_client_event::<LotSpawn>(EventType::Unordered)
            .add_mapped_client_event::<LotMove>(EventType::Ordered)
            .add_mapped_client_event::<LotDespawn>(EventType::Unordered)
            .add_server_event::<LotEventConfirmed>(EventType::Unordered)
            .add_systems(
                PreUpdate,
                (Self::init_system, Self::vertices_update_system)
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                Update,
                (
                    Self::spawn_system,
                    Self::movement_system,
                    Self::despawn_system,
                )
                    .run_if(has_authority()),
            )
            .add_systems(
                PostUpdate,
                Self::ignore_transform_system.before(ServerSet::Send),
            );
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
        changed_lots: Query<(&Handle<Polyline>, Ref<LotVertices>)>,
    ) {
        for (polyline_handle, vertices) in &changed_lots {
            if vertices.is_changed() && !vertices.is_added() {
                let polyline = polylines
                    .get_mut(polyline_handle)
                    .expect("polyline should be spawned on init");
                polyline.vertices = vertices.to_plain();
            }
        }
    }

    fn spawn_system(
        mut commands: Commands,
        mut spawn_events: EventReader<FromClient<LotSpawn>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in spawn_events.read().cloned() {
            commands.entity(event.city_entity).with_children(|parent| {
                parent.spawn(LotBundle::new(event.vertices));
            });
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
        }
    }

    fn movement_system(
        mut move_events: EventReader<FromClient<LotMove>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
        mut lots: Query<&mut LotVertices>,
    ) {
        for FromClient { client_id, event } in move_events.read().copied() {
            match lots.get_mut(event.entity) {
                Ok(mut vertices) => {
                    for vertex in &mut vertices.0 {
                        *vertex += event.offset;
                    }
                    confirm_events.send(ToClients {
                        mode: SendMode::Direct(client_id),
                        event: LotEventConfirmed,
                    });
                }
                Err(e) => error!("unable to apply lot movement: {e}"),
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<FromClient<LotDespawn>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in despawn_events.read().copied() {
            commands.entity(event.0).despawn();
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
        }
    }

    fn ignore_transform_system(mut commands: Commands, lots: Query<Entity, Added<LotVertices>>) {
        for entity in &lots {
            commands
                .entity(entity)
                .insert(Ignored::<Transform>::default());
        }
    }
}

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, States,
)]
pub(crate) enum LotTool {
    #[default]
    Create,
    Move,
}

impl LotTool {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Create => "✏",
            Self::Move => "↔",
        }
    }
}

#[derive(Bundle)]
struct LotBundle {
    vertices: LotVertices,
    parent_sync: ParentSync,
    replication: Replication,
}

impl LotBundle {
    fn new(vertices: Vec<Vec2>) -> Self {
        Self {
            vertices: LotVertices(vertices),
            parent_sync: Default::default(),
            replication: Replication,
        }
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(super) struct LotVertices(Vec<Vec2>);

/// Contains a family entity that owns the lot.
#[derive(Component)]
pub(crate) struct LotFamily(pub(crate) Entity);

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
    pub(super) fn contains_point(&self, point: Vec2) -> bool {
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
            width: 25.0,
            color: Color::WHITE,
            perspective: true,
            ..Default::default()
        });
        Self(material_handle)
    }
}

#[derive(Clone, Deserialize, Event, Serialize)]
struct LotSpawn {
    vertices: Vec<Vec2>,
    city_entity: Entity,
}

impl MapNetworkEntities for LotSpawn {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.city_entity = mapper.map(self.city_entity);
    }
}

#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct LotMove {
    entity: Entity,
    offset: Vec2,
}

impl MapNetworkEntities for LotMove {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.entity = mapper.map(self.entity);
    }
}

#[derive(Clone, Copy, Event, Deserialize, Serialize)]
struct LotDespawn(Entity);

impl MapNetworkEntities for LotDespawn {
    fn map_entities<T: Mapper>(&mut self, mapper: &mut T) {
        self.0 = mapper.map(self.0);
    }
}

#[derive(Deserialize, Event, Serialize)]
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
