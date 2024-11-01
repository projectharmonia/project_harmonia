pub mod placing_road;
pub(crate) mod road_mesh;

use avian3d::prelude::*;
use bevy::{
    asset::AssetPath, ecs::entity::MapEntities, prelude::*, render::view::NoFrustumCulling,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::{
    asset::info::road_info::RoadInfo,
    core::GameState,
    game_world::{
        city::CityMode,
        commands_history::{
            CommandConfirmation, CommandId, CommandRequest, ConfirmableCommand, EntityRecorder,
            PendingCommand,
        },
        spline::{
            dynamic_mesh::DynamicMesh, PointKind, SplineConnections, SplinePlugin, SplineSegment,
        },
        Layer,
    },
    math::segment::Segment,
};
use placing_road::PlacingRoadPlugin;

pub(crate) struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlacingRoadPlugin)
            .add_sub_state::<RoadTool>()
            .enable_state_scoped_entities::<RoadTool>()
            .register_type::<Road>()
            .register_type::<RoadData>()
            .replicate::<Road>()
            .add_mapped_client_event::<CommandRequest<RoadCommand>>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::apply_command
                        .run_if(server_or_singleplayer)
                        .before(ServerSet::StoreHierarchy),
                    Self::update_meshes.after(SplinePlugin::update_connections),
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl RoadPlugin {
    fn init(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        roads_info: Res<Assets<RoadInfo>>,
        roads: Query<(Entity, &Road), Without<Handle<Mesh>>>,
    ) {
        for (entity, road) in &roads {
            let info_handle = asset_server
                .get_handle(&road.0)
                .expect("info should be preloaded");
            let info = roads_info.get(&info_handle).unwrap();
            debug!("initializing road '{}' for `{entity}`", road.0);

            commands.entity(entity).insert((
                Name::new("Road"),
                RoadData::new(info),
                Collider::default(),
                CollisionLayers::new(Layer::Road, [Layer::Wall, Layer::PlacingWall]),
                NoFrustumCulling,
                PbrBundle {
                    material: asset_server.load(info.material.clone()),
                    mesh: meshes.add(DynamicMesh::create_empty()),
                    ..Default::default()
                },
            ));
        }
    }

    fn update_meshes(
        mut meshes: ResMut<Assets<Mesh>>,
        mut changed_roads: Query<
            (
                &Handle<Mesh>,
                Ref<SplineSegment>,
                &SplineConnections,
                &RoadData,
                &mut Collider,
            ),
            Changed<SplineConnections>,
        >,
    ) {
        for (mesh_handle, segment, connections, road_data, mut collider) in &mut changed_roads {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("road handles should be valid");

            trace!("regenerating road mesh");
            let mut dyn_mesh = DynamicMesh::take(mesh);
            road_mesh::generate(&mut dyn_mesh, *segment, connections, road_data.half_width);
            dyn_mesh.apply(mesh);

            if segment.is_changed() || collider.is_added() {
                trace!("regenerating road collision");
                *collider = road_mesh::generate_collider(*segment, road_data.half_width);
            }
        }
    }

    fn apply_command(
        mut commands: Commands,
        mut request_events: EventReader<FromClient<CommandRequest<RoadCommand>>>,
        mut confirm_events: EventWriter<ToClients<CommandConfirmation>>,
        mut roads: Query<&mut SplineSegment, With<Road>>,
    ) {
        for FromClient { client_id, event } in request_events.read().cloned() {
            // TODO: validate if command can be applied.
            let mut confirmation = CommandConfirmation::new(event.id);
            match event.command {
                RoadCommand::Create {
                    city_entity,
                    info_path,
                    segment,
                } => {
                    info!("`{client_id:?}` spawns road");
                    commands.entity(city_entity).with_children(|parent| {
                        let entity = parent
                            .spawn(RoadBundle::new(info_path.clone(), segment))
                            .id();
                        confirmation.entity = Some(entity);
                    });
                }
                RoadCommand::MovePoint {
                    entity,
                    kind,
                    point,
                } => match roads.get_mut(entity) {
                    Ok(mut segment) => {
                        info!("`{client_id:?}` moves `{kind:?}` for road `{entity}`");
                        match kind {
                            PointKind::Start => segment.start = point,
                            PointKind::End => segment.end = point,
                        }
                    }
                    Err(e) => error!("unable to move road `{entity}`: {e}"),
                },
                RoadCommand::Delete { entity } => {
                    info!("`{client_id:?}` removes road `{entity}`");
                    commands.entity(entity).despawn();
                }
            }

            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: confirmation,
            });
        }
    }
}

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, SubStates,
)]
#[source(CityMode = CityMode::Roads)]
pub enum RoadTool {
    #[default]
    Create,
    Move,
}

impl RoadTool {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Create => "✏",
            Self::Move => "↔",
        }
    }
}

#[derive(Bundle)]
struct RoadBundle {
    road: Road,
    spline_segment: SplineSegment,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl RoadBundle {
    fn new(info_path: AssetPath<'static>, segment: Segment) -> Self {
        Self {
            road: Road(info_path),
            spline_segment: SplineSegment(segment),
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

/// Stores path to the road info.
#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
struct Road(AssetPath<'static>);

/// Stores road information needed at runtime from [`RoadInfo`].
#[derive(Component, Reflect)]
#[reflect(Component)]
struct RoadData {
    half_width: f32,
}

impl RoadData {
    fn new(info: &RoadInfo) -> Self {
        Self {
            half_width: info.half_width,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
enum RoadCommand {
    Create {
        city_entity: Entity,
        info_path: AssetPath<'static>,
        segment: Segment,
    },
    MovePoint {
        entity: Entity,
        kind: PointKind,
        point: Vec2,
    },
    Delete {
        entity: Entity,
    },
}

impl PendingCommand for RoadCommand {
    fn apply(
        self: Box<Self>,
        id: CommandId,
        mut recorder: EntityRecorder,
        world: &mut World,
    ) -> Box<dyn ConfirmableCommand> {
        let reverse_command = match *self {
            Self::Create { .. } => Self::Delete {
                // Correct entity will be set after the server confirmation.
                entity: Entity::PLACEHOLDER,
            },
            Self::MovePoint { entity, kind, .. } => {
                let segment = world.get::<SplineSegment>(entity).unwrap();
                let point = match kind {
                    PointKind::Start => segment.start,
                    PointKind::End => segment.end,
                };
                Self::MovePoint {
                    entity,
                    kind,
                    point,
                }
            }
            Self::Delete { entity } => {
                recorder.record(entity);
                let entity = world.entity(entity);
                let road = entity.get::<Road>().unwrap();
                let segment = **entity.get::<SplineSegment>().unwrap();
                let city_entity = **entity.get::<Parent>().unwrap();
                Self::Create {
                    city_entity,
                    info_path: road.0.clone(),
                    segment,
                }
            }
        };

        world.send_event(CommandRequest { id, command: *self });

        Box::new(reverse_command)
    }
}

impl ConfirmableCommand for RoadCommand {
    fn confirm(
        mut self: Box<Self>,
        mut recorder: EntityRecorder,
        confirmation: CommandConfirmation,
    ) -> Box<dyn PendingCommand> {
        if let Self::Delete { entity } = &mut *self {
            *entity = confirmation
                .entity
                .expect("confirmation for road creation should contain an entity");
            recorder.record(*entity);
        }

        self
    }
}

impl MapEntities for RoadCommand {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        match self {
            Self::Create { .. } => (),
            Self::MovePoint { entity, .. } => *entity = entity_mapper.map_entity(*entity),
            Self::Delete { entity } => *entity = entity_mapper.map_entity(*entity),
        };
    }
}
