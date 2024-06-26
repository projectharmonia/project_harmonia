pub(crate) mod creating_lot;
pub(crate) mod moving_lot;

use bevy::{ecs::entity::MapEntities, prelude::*};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::{game_world::GameWorld, math::polygon::Polygon};
use creating_lot::CreatingLotPlugin;
use moving_lot::MovingLotPlugin;

pub(super) struct LotPlugin;

impl Plugin for LotPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<LotTool>()
            .add_plugins((CreatingLotPlugin, MovingLotPlugin))
            .register_type::<Vec<Vec2>>()
            .register_type::<LotVertices>()
            .replicate::<LotVertices>()
            .add_mapped_client_event::<LotCreate>(ChannelKind::Unordered)
            .add_mapped_client_event::<LotMove>(ChannelKind::Ordered)
            .add_mapped_client_event::<LotDelete>(ChannelKind::Unordered)
            .add_server_event::<LotEventConfirmed>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(resource_exists::<GameWorld>),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::draw_lines.run_if(resource_exists::<GameWorld>),
                    (
                        Self::create.before(ServerSet::StoreHierarchy),
                        Self::apply_movement,
                        Self::delete,
                    )
                        .run_if(has_authority),
                ),
            );
    }
}

impl LotPlugin {
    fn init(mut commands: Commands, spawned_lots: Query<Entity, Added<LotVertices>>) {
        for entity in &spawned_lots {
            commands.entity(entity).insert(SpatialBundle::default());
        }
    }

    fn draw_lines(mut gizmos: Gizmos, lots: Query<(&GlobalTransform, &LotVertices)>) {
        for (transform, vertices) in &lots {
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
            commands.entity(event.0).despawn_recursive();
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: LotEventConfirmed,
            });
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
pub(super) struct LotVertices(Polygon);

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
