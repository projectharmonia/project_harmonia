pub(crate) mod creating_lot;
pub(crate) mod moving_lot;

use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_polyline::prelude::*;
use bevy_replicon::prelude::*;
use derive_more::Display;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use tap::TapFallible;

use super::{
    family::{Family, FamilyMode},
    game_state::GameState,
    game_world::{parent_sync::ParentSync, WorldState},
    ground::Ground,
    task::{TaskActivation, TaskList, TaskRequest, TaskRequestKind},
};
use creating_lot::CreatingLotPlugin;
use moving_lot::MovingLotPlugin;

pub(super) struct LotPlugin;

impl Plugin for LotPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<LotTool>()
            .add_plugin(CreatingLotPlugin)
            .add_plugin(MovingLotPlugin)
            .register_type::<Vec<Vec2>>()
            .replicate::<LotVertices>()
            .not_replicate_if_present::<Transform, LotVertices>()
            .add_mapped_client_event::<LotSpawn>()
            .add_mapped_client_event::<LotMove>()
            .add_mapped_client_event::<LotDespawn>()
            .add_server_event::<LotEventConfirmed>()
            .add_system(
                Self::tasks_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (Self::vertices_update_system, Self::init_system)
                    .in_set(OnUpdate(WorldState::InWorld)),
            )
            .add_systems(
                (
                    Self::buying_system,
                    Self::spawn_system,
                    Self::movement_system,
                    Self::despawn_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl LotPlugin {
    fn tasks_system(
        mut ground: Query<&mut TaskList, (With<Ground>, Added<TaskList>)>,
        lots: Query<&LotVertices, Without<LotFamily>>,
    ) {
        if let Ok(mut task_list) = ground.get_single_mut() {
            let position = task_list.position.xz();
            if lots
                .iter()
                .any(|vertices| vertices.contains_point(position))
            {
                task_list.tasks.push(TaskRequestKind::Buy);
            }
        }
    }

    fn buying_system(
        mut commands: Commands,
        mut activation_events: EventReader<TaskActivation>,
        lots: Query<(Entity, &LotVertices), Without<LotFamily>>,
        actors: Query<&Family>,
    ) {
        for TaskActivation { entity, task } in activation_events.iter().copied() {
            if let TaskRequest::Buy(position) = task {
                let family = actors.get(entity).expect("actor should belong to a family");
                if let Some((lot_entity, _)) = lots
                    .iter()
                    .find(|(_, vertices)| vertices.contains_point(position))
                {
                    commands.entity(lot_entity).insert(LotFamily(family.0));
                }
            }
        }
    }

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
        for FromClient { client_id, event } in spawn_events.iter().cloned() {
            commands.spawn(LotBundle::new(event.vertices, event.city_entity));
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
        for FromClient { client_id, event } in move_events.iter().copied() {
            if let Ok(mut vertices) = lots
                .get_mut(event.entity)
                .tap_err(|e| error!("unable to apply lot movement from client {client_id}: {e}"))
            {
                for vertex in &mut vertices.0 {
                    *vertex += event.offset;
                }
                confirm_events.send(ToClients {
                    mode: SendMode::Direct(client_id),
                    event: LotEventConfirmed,
                });
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventReader<FromClient<LotDespawn>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in despawn_events.iter().copied() {
            commands.entity(event.0).despawn();
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, States)]
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
    fn new(vertices: Vec<Vec2>, city_entity: Entity) -> Self {
        Self {
            vertices: LotVertices(vertices),
            parent_sync: ParentSync(city_entity),
            replication: Replication,
        }
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(super) struct LotVertices(Vec<Vec2>);

/// Contains a family entity that owns the lot.
#[derive(Component)]
struct LotFamily(Entity);

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

impl MapEntities for LotSpawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.city_entity = entity_map.get(self.city_entity)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct LotMove {
    entity: Entity,
    offset: Vec2,
}

impl MapEntities for LotMove {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.entity = entity_map.get(self.entity)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct LotDespawn(Entity);

impl MapEntities for LotDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
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
