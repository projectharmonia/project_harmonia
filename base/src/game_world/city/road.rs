pub mod placing_road;
pub(crate) mod road_mesh;

use avian3d::prelude::*;
use bevy::{asset::AssetPath, ecs::entity::MapEntities, prelude::*};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use crate::{
    asset::manifest::road_manifest::RoadManifest,
    core::GameState,
    dynamic_mesh::DynamicMesh,
    game_world::{
        city::CityMode,
        commands_history::{
            CommandConfirmation, CommandId, CommandRequest, ConfirmableCommand, EntityRecorder,
            PendingCommand,
        },
        segment::{PointKind, Segment, SegmentConnections, SegmentPlugin},
        Layer,
    },
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
            .add_observer(Self::init)
            .add_systems(
                PostUpdate,
                (
                    Self::apply_command
                        .run_if(server_or_singleplayer)
                        .before(ServerSet::StoreHierarchy),
                    Self::update_meshes.after(SegmentPlugin::update_connections),
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl RoadPlugin {
    fn init(
        trigger: Trigger<OnAdd, Road>,
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        manifests: Res<Assets<RoadManifest>>,
        mut roads: Query<(
            &Road,
            &mut RoadData,
            &mut Mesh3d,
            &mut MeshMaterial3d<StandardMaterial>,
        )>,
    ) {
        let (road, mut road_data, mut mesh, mut material) =
            roads.get_mut(trigger.entity()).unwrap();
        let Some(manifest_handle) = asset_server.get_handle(&**road) else {
            error!("'{}' is missing, ignoring", &**road);
            return;
        };

        debug!("initializing road '{}' for `{}`", &**road, trigger.entity());

        let manifest = manifests
            .get(&manifest_handle)
            .unwrap_or_else(|| panic!("'{:?}' should be loaded", &**road));

        road_data.half_width = manifest.half_width;
        **mesh = meshes.add(DynamicMesh::create_empty());
        **material = asset_server.load(manifest.material.clone());
    }

    fn update_meshes(
        mut meshes: ResMut<Assets<Mesh>>,
        mut changed_roads: Query<
            (
                &Mesh3d,
                Ref<Segment>,
                &SegmentConnections,
                &RoadData,
                &mut Collider,
            ),
            Changed<SegmentConnections>,
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
        mut roads: Query<&mut Segment, With<Road>>,
    ) {
        for FromClient { client_id, event } in request_events.read().cloned() {
            // TODO: validate if command can be applied.
            let mut confirmation = CommandConfirmation::new(event.id);
            match event.command {
                RoadCommand::Create {
                    city_entity,
                    manifest_path,
                    segment,
                } => {
                    info!("`{client_id:?}` spawns road");
                    commands.entity(city_entity).with_children(|parent| {
                        let entity = parent.spawn((Road(manifest_path.clone()), segment)).id();
                        confirmation.entity = Some(entity);
                    });
                }
                RoadCommand::EditPoint {
                    entity,
                    kind,
                    point,
                } => match roads.get_mut(entity) {
                    Ok(mut segment) => {
                        info!("`{client_id:?}` edits `{kind:?}` for road `{entity}`");
                        segment.set_point(kind, point);
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

#[derive(Clone, Component, Copy, Debug, Default, EnumIter, Eq, Hash, PartialEq, SubStates)]
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

/// Stores path to the road manifest.
#[derive(Component, Deserialize, Reflect, Serialize, Deref)]
#[reflect(Component)]
#[require(
    Name(|| Name::new("Road")),
    Segment,
    ParentSync,
    Replicated,
    RoadData,
    Mesh3d,
    MeshMaterial3d::<StandardMaterial>,
    Collider,
    CollisionLayers(|| CollisionLayers::new(Layer::Road, [Layer::Wall, Layer::PlacingWall])),
)]
struct Road(AssetPath<'static>);

/// Stores road information needed at runtime from [`RoadManifest`].
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct RoadData {
    half_width: f32,
}

#[derive(Serialize, Deserialize, Clone)]
enum RoadCommand {
    Create {
        city_entity: Entity,
        manifest_path: AssetPath<'static>,
        segment: Segment,
    },
    EditPoint {
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
            Self::EditPoint { entity, kind, .. } => {
                let segment = world.get::<Segment>(entity).unwrap();
                let point = segment.point(kind);
                Self::EditPoint {
                    entity,
                    kind,
                    point,
                }
            }
            Self::Delete { entity } => {
                recorder.record(entity);
                let entity = world.entity(entity);
                let road = entity.get::<Road>().unwrap();
                let segment = *entity.get::<Segment>().unwrap();
                let city_entity = **entity.get::<Parent>().unwrap();
                Self::Create {
                    city_entity,
                    manifest_path: road.0.clone(),
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
            Self::EditPoint { entity, .. } => *entity = entity_mapper.map_entity(*entity),
            Self::Delete { entity } => *entity = entity_mapper.map_entity(*entity),
        };
    }
}
