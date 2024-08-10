pub mod creating_road;
pub(crate) mod road_mesh;

use bevy::{
    asset::AssetPath,
    ecs::entity::MapEntities,
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology, view::NoFrustumCulling},
};
use bevy_replicon::prelude::*;
use road_mesh::RoadMesh;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use crate::{
    asset::info::road_info::RoadInfo,
    core::GameState,
    game_world::{
        city::CityMode,
        spline::{SplineConnections, SplinePlugin, SplineSegment},
    },
};
use creating_road::CreatingRoadPlugin;

pub(crate) struct RoadPlugin;

impl Plugin for RoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(CreatingRoadPlugin)
            .register_type::<Road>()
            .replicate::<Road>()
            .add_mapped_client_event::<RoadCreate>(ChannelKind::Unordered)
            .add_server_event::<RoadCreateConfirmed>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::create
                        .run_if(has_authority)
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

            let mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<Vec3>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, Vec::<Vec2>::new())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_inserted_indices(Indices::U32(Vec::new()));

            let mut entity = commands.entity(entity);
            entity.insert((
                NoFrustumCulling,
                PbrBundle {
                    material: asset_server.load(info.material.clone()),
                    mesh: meshes.add(mesh),
                    ..Default::default()
                },
            ));
        }
    }

    fn update_meshes(
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        roads_info: Res<Assets<RoadInfo>>,
        mut changed_roads: Query<
            (&Handle<Mesh>, &SplineSegment, &SplineConnections, &Road),
            Or<(Changed<SplineConnections>, Added<Handle<Mesh>>)>, // `Added` is needed to run after scene load.
        >,
    ) {
        for (mesh_handle, segment, connections, road) in &mut changed_roads {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("road handles should be valid");

            let info_handle = asset_server
                .get_handle(&road.0)
                .expect("info should be preloaded");
            let info = roads_info.get(&info_handle).unwrap();

            trace!("regenerating road mesh");
            let mut road_mesh = RoadMesh::take(mesh);
            road_mesh.generate(*segment, connections, info.half_width);
            road_mesh.apply(mesh);
        }
    }

    fn create(
        mut commands: Commands,
        mut create_events: EventReader<FromClient<RoadCreate>>,
        mut confirm_events: EventWriter<ToClients<RoadCreateConfirmed>>,
    ) {
        for FromClient { client_id, event } in create_events.read() {
            // TODO: Validate if the road can be spawned.
            info!("`{client_id:?}` spawns road");
            confirm_events.send(ToClients {
                mode: SendMode::Direct(*client_id),
                event: RoadCreateConfirmed,
            });
            commands.entity(event.city_entity).with_children(|parent| {
                parent.spawn(RoadBundle::new(event.info_path.clone(), event.segment));
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
}

impl RoadTool {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Create => "✏",
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
    fn new(info_path: AssetPath<'static>, segment: SplineSegment) -> Self {
        Self {
            road: Road(info_path),
            spline_segment: segment,
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

/// Stores path to the road info.
#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
struct Road(AssetPath<'static>);

/// Client event to request a road creation.
#[derive(Clone, Deserialize, Event, Serialize)]
struct RoadCreate {
    city_entity: Entity,
    info_path: AssetPath<'static>,
    segment: SplineSegment,
}

impl MapEntities for RoadCreate {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.city_entity = entity_mapper.map_entity(self.city_entity);
    }
}

#[derive(Deserialize, Event, Serialize)]
struct RoadCreateConfirmed;
