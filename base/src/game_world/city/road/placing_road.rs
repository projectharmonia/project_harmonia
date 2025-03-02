use avian3d::prelude::*;
use bevy::{
    color::palettes::css::{RED, WHITE},
    math::Vec3Swizzles,
    prelude::*,
    render::view::NoFrustumCulling,
};
use bevy_enhanced_input::prelude::*;

use super::{Road, RoadData, RoadTool};
use crate::{
    alpha_color::{self, AlphaColor},
    asset::manifest::road_manifest::RoadManifest,
    dynamic_mesh::DynamicMesh,
    game_world::{
        city::{road::RoadCommand, ActiveCity, CityMode},
        commands_history::{CommandsHistory, PendingDespawn},
        segment::{
            placing_segment::{ConfirmSegment, DeleteSegment, PlacingSegment},
            PointKind, Segment,
        },
        Layer,
    },
    ghost::Ghost,
};

pub(super) struct PlacingRoadPlugin;

impl Plugin for PlacingRoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(pick.never_param_warn())
            .add_observer(spawn.never_param_warn())
            .add_observer(delete.never_param_warn())
            .add_observer(confirm.never_param_warn())
            .add_systems(
                PostUpdate,
                update_alpha
                    .never_param_warn()
                    .before(alpha_color::update_materials)
                    .run_if(in_state(CityMode::Roads)),
            );
    }
}

fn pick(
    mut trigger: Trigger<Pointer<Click>>,
    road_tool: Res<State<RoadTool>>,
    mut commands: Commands,
    manifests: Res<Assets<RoadManifest>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    roads: Query<(
        Entity,
        &Parent,
        &MeshMaterial3d<StandardMaterial>,
        &Road,
        &Segment,
    )>,
    placing_roads: Query<(), With<PlacingRoad>>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    if *road_tool != RoadTool::Move {
        return;
    }
    if !placing_roads.is_empty() {
        return;
    }
    let Ok((entity, parent, material, road, &segment)) = roads.get(trigger.entity()) else {
        return;
    };
    trigger.propagate(false);

    let manifest_handle = asset_server
        .get_handle(&road.0)
        .expect("manifest should be preloaded");
    let manifest = manifests.get(&manifest_handle).unwrap();

    let point = trigger.hit.position.unwrap();
    let point_kind = if segment.start.distance(point.xz()) < manifest.half_width {
        PointKind::Start
    } else if segment.end.distance(point.xz()) < manifest.half_width {
        PointKind::End
    } else {
        return;
    };

    info!("picking `{point_kind:?}` for `{entity}`");
    commands.entity(**parent).with_children(|parent| {
        parent.spawn((
            Ghost::new(entity),
            PlacingRoad::EditPoint { entity },
            RoadData {
                half_width: manifest.half_width,
            },
            segment,
            PlacingSegment {
                point_kind,
                snap_offset: manifest.half_width,
            },
            material.clone(),
            Mesh3d(meshes.add(DynamicMesh::create_empty())),
        ));
    });
}

fn spawn(
    mut trigger: Trigger<Pointer<Click>>,
    road_tool: Res<State<RoadTool>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    road_manifests: Res<Assets<RoadManifest>>,
    placing_id: Option<Res<SpawnRoadId>>,
    city_entity: Single<Entity, With<ActiveCity>>,
    roads: Query<(&Parent, &Segment), With<Road>>,
    placing_roads: Query<(), With<PlacingRoad>>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    if *road_tool != RoadTool::Create {
        return;
    }
    if !placing_roads.is_empty() {
        return;
    }
    let Some(point) = trigger.hit.position else {
        // Consider only world clicking.
        return;
    };
    let Some(placing_id) = placing_id else {
        return;
    };

    trigger.propagate(false);

    let manifest = road_manifests
        .get(placing_id.0)
        .expect("manifests should be preloaded");

    // Use an existing point if it is within the half width distance.
    let snapped_point = roads
        .iter()
        .filter(|(parent, _)| ***parent == *city_entity)
        .flat_map(|(_, segment)| segment.points())
        .find(|vertex| vertex.distance(point.xz()) < manifest.half_width)
        .unwrap_or(point.xz());

    info!("spawning new road");
    commands.entity(*city_entity).with_children(|parent| {
        parent.spawn((
            PlacingRoad::Spawning(placing_id.0),
            RoadData {
                half_width: manifest.half_width,
            },
            Segment::splat(snapped_point),
            PlacingSegment {
                point_kind: PointKind::End,
                snap_offset: manifest.half_width,
            },
            Mesh3d(meshes.add(DynamicMesh::create_empty())),
            MeshMaterial3d::<StandardMaterial>(asset_server.load(manifest.material.clone())),
        ));
    });
}

fn update_alpha(
    placing_road: Single<
        (&mut AlphaColor, &CollidingEntities),
        (Changed<CollidingEntities>, With<PlacingRoad>),
    >,
) {
    let (mut alpha, colliding_entities) = placing_road.into_inner();
    if colliding_entities.is_empty() {
        **alpha = WHITE.into();
    } else {
        **alpha = RED.into();
    };
}

fn delete(
    trigger: Trigger<Completed<DeleteSegment>>,
    mut commands: Commands,
    mut history: CommandsHistory,
    placing_road: Single<&PlacingRoad>,
) {
    info!("deleting road");
    if let PlacingRoad::EditPoint { entity } = **placing_road {
        let command_id = history.push_pending(RoadCommand::Delete { entity });
        commands
            .entity(trigger.entity())
            .insert(PendingDespawn { command_id })
            .remove::<(PlacingRoad, Segment)>();
    } else {
        commands.entity(trigger.entity()).despawn_recursive();
    }
}

fn confirm(
    trigger: Trigger<Completed<ConfirmSegment>>,
    mut commands: Commands,
    mut history: CommandsHistory,
    asset_server: Res<AssetServer>,
    placing_road: Single<(&Parent, &Segment, &PlacingRoad, &PlacingSegment)>,
) {
    let (parent, &segment, &placing_road, placing_segment) = *placing_road;

    info!("configrming {placing_road:?}");
    let command_id = match placing_road {
        PlacingRoad::Spawning(id) => {
            let manifest_path = asset_server
                .get_path(id)
                .expect("manifest should always come from file");
            history.push_pending(RoadCommand::Create {
                city_entity: **parent,
                manifest_path: manifest_path.into_owned(),
                segment,
            })
        }
        PlacingRoad::EditPoint { entity } => {
            let point = segment.point(placing_segment.point_kind);
            history.push_pending(RoadCommand::EditPoint {
                entity,
                kind: placing_segment.point_kind,
                point,
            })
        }
    };

    commands
        .entity(trigger.entity())
        .insert(PendingDespawn { command_id })
        .remove::<(PlacingRoad, Segment)>();
}

/// ID to spawn new roads with.
///
/// Spawning won't start until this resource is inserted.
#[derive(Resource)]
pub struct SpawnRoadId(pub AssetId<RoadManifest>);

#[derive(Debug, Clone, Copy, Component)]
#[require(
    Name(|| Name::new("Placing road")),
    RoadData,
    // Looks like AABB is not recalculated when we edit the mesh.
    // But we don't need to cull currently placed road anyway.
    NoFrustumCulling,
    AlphaColor(|| AlphaColor(WHITE.into())),
    Mesh3d,
    MeshMaterial3d::<StandardMaterial>,
    Collider,
    CollisionLayers(|| CollisionLayers::new(
        Layer::PlacingRoad,
        [Layer::Wall, Layer::PlacingWall],
    )),
)]
enum PlacingRoad {
    Spawning(AssetId<RoadManifest>),
    EditPoint { entity: Entity },
}
