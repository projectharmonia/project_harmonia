pub mod creating_lot;
pub mod moving_lot;

use bevy::{ecs::entity::MapEntities, prelude::*};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::{
    game_world::{city::CityMode, WorldState},
    math::polygon::Polygon,
};
use creating_lot::CreatingLotPlugin;
use moving_lot::MovingLotPlugin;

pub(super) struct LotPlugin;

impl Plugin for LotPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<LotTool>()
            .enable_state_scoped_entities::<LotTool>()
            .add_plugins((CreatingLotPlugin, MovingLotPlugin))
            .register_type::<Vec<Vec2>>()
            .register_type::<LotVertices>()
            .replicate::<LotVertices>()
            .add_mapped_client_event::<LotCreate>(ChannelKind::Unordered)
            .add_mapped_client_event::<LotMove>(ChannelKind::Ordered)
            .add_mapped_client_event::<LotDelete>(ChannelKind::Unordered)
            .add_server_event::<LotEventConfirmed>(ChannelKind::Unordered)
            .add_systems(
                PostUpdate,
                (
                    Self::draw_lines
                        .run_if(in_state(WorldState::City).or_else(in_state(WorldState::Family))),
                    (
                        Self::create.before(ServerSet::StoreHierarchy),
                        Self::apply_movement,
                        Self::delete,
                    )
                        .run_if(server_or_singleplayer),
                ),
            );
    }
}

impl LotPlugin {
    fn draw_lines(
        mut gizmos: Gizmos,
        lots: Query<(&Parent, &LotVertices)>,
        cities: Query<&GlobalTransform>,
    ) {
        for (parent, vertices) in &lots {
            let transform = cities.get(**parent).unwrap();
            let points_iter = vertices
                .iter()
                .map(|vertex| Vec3::new(vertex.x, 0.0, vertex.y))
                .map(|point| transform.transform_point(point));
            gizmos.linestrip(points_iter, Color::WHITE);
        }
    }

    fn create(
        mut commands: Commands,
        mut create_events: EventReader<FromClient<LotCreate>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in create_events.read().cloned() {
            info!("`{client_id:?}` creates lot");
            commands.entity(event.city_entity).with_children(|parent| {
                parent.spawn(LotBundle::new(event.polygon));
            });
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
        }
    }

    fn apply_movement(
        mut move_events: EventReader<FromClient<LotMove>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
        mut lots: Query<&mut LotVertices>,
    ) {
        for FromClient { client_id, event } in move_events.read().copied() {
            match lots.get_mut(event.entity) {
                Ok(mut vertices) => {
                    info!("`{client_id:?}` moves lot `{:?}`", event.entity);
                    for vertex in vertices.iter_mut() {
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

    fn delete(
        mut commands: Commands,
        mut delete_events: EventReader<FromClient<LotDelete>>,
        mut confirm_events: EventWriter<ToClients<LotEventConfirmed>>,
    ) {
        for FromClient { client_id, event } in delete_events.read().copied() {
            info!("`{client_id:?}` deletes lot `{:?}`", event.0);
            commands.entity(event.0).despawn_recursive();
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
        }
    }
}

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, SubStates,
)]
#[source(CityMode = CityMode::Lots)]
pub enum LotTool {
    #[default]
    Create,
    Move,
}

impl LotTool {
    pub fn glyph(self) -> &'static str {
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
    replication: Replicated,
}

impl LotBundle {
    fn new(polygon: Polygon) -> Self {
        Self {
            vertices: LotVertices(polygon),
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

#[derive(Clone, Component, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct LotVertices(Polygon);

/// Contains a family entity that owns the lot.
#[derive(Component)]
#[allow(dead_code)]
pub(crate) struct LotFamily(pub(crate) Entity);

#[derive(Clone, Deserialize, Event, Serialize)]
struct LotCreate {
    polygon: Polygon,
    city_entity: Entity,
}

impl MapEntities for LotCreate {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.city_entity = entity_mapper.map_entity(self.city_entity);
    }
}

#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct LotMove {
    entity: Entity,
    offset: Vec2,
}

impl MapEntities for LotMove {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.entity = entity_mapper.map_entity(self.entity);
    }
}

#[derive(Clone, Copy, Event, Deserialize, Serialize)]
struct LotDelete(Entity);

impl MapEntities for LotDelete {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}

#[derive(Deserialize, Event, Serialize)]
struct LotEventConfirmed;

#[derive(Component)]
struct UnconfirmedLot;
