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
    alpha_color::{AlphaColor, AlphaColorPlugin},
    asset::info::road_info::RoadInfo,
    common_conditions::observer_in_state,
    dynamic_mesh::DynamicMesh,
    game_world::{
        city::{road::RoadCommand, ActiveCity, CityMode},
        commands_history::{CommandsHistory, PendingDespawn},
        picking::Clicked,
        segment::{
            placing_segment::{CancelSegment, ConfirmSegment, DeleteSegment, PlacingSegment},
            PointKind, Segment,
        },
        Layer,
    },
    ghost::Ghost,
};

pub(super) struct PlacingRoadPlugin;

impl Plugin for PlacingRoadPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::pick)
            .observe(Self::spawn)
            .observe(Self::delete)
            .observe(Self::cancel)
            .observe(Self::confirm)
            .add_systems(
                PostUpdate,
                Self::update_alpha
                    .before(AlphaColorPlugin::update_materials)
                    .after(PhysicsSet::StepSimulation)
                    .run_if(in_state(CityMode::Roads)),
            );
    }
}

impl PlacingRoadPlugin {
    fn pick(
        trigger: Trigger<Clicked>,
        road_tool: Option<Res<State<RoadTool>>>,
        mut commands: Commands,
        roads_info: Res<Assets<RoadInfo>>,
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        roads: Query<(Entity, &Parent, &Handle<StandardMaterial>, &Road, &Segment)>,
    ) {
        if !observer_in_state(road_tool, RoadTool::Move) {
            return;
        }

        let Ok((entity, parent, material, road, &segment)) = roads.get(trigger.entity()) else {
            return;
        };

        let info_handle = asset_server
            .get_handle(&road.0)
            .expect("info should be preloaded");
        let info = roads_info.get(&info_handle).unwrap();

        let point = trigger.event().xz();
        let point_kind = if segment.start.distance(point) < info.half_width {
            PointKind::Start
        } else if segment.end.distance(point) < info.half_width {
            PointKind::End
        } else {
            return;
        };

        info!("picking `{point_kind:?}` for `{entity}`");
        commands.entity(**parent).with_children(|parent| {
            parent.spawn((
                Ghost::new(entity),
                PlacingRoadBundle::new(
                    PlacingRoad::EditPoint { entity },
                    info.half_width,
                    segment,
                    PlacingSegment {
                        point_kind,
                        snap_offset: info.half_width,
                    },
                    material.clone(),
                    meshes.add(DynamicMesh::create_empty()),
                ),
            ));
        });
    }

    fn spawn(
        trigger: Trigger<Clicked>,
        road_tool: Option<Res<State<RoadTool>>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        asset_server: Res<AssetServer>,
        roads_info: Res<Assets<RoadInfo>>,
        placing_id: Option<Res<SpawnRoadId>>,
        roads: Query<(&Parent, &Segment), With<Road>>,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if !observer_in_state(road_tool, RoadTool::Create) {
            return;
        }

        let Some(placing_id) = placing_id else {
            return;
        };

        let city_entity = cities.single();
        let info = roads_info
            .get(placing_id.0)
            .expect("info should be preloaded");

        // Use an existing point if it is within the half width distance.
        let point = trigger.event().xz();
        let snapped_point = roads
            .iter()
            .filter(|(parent, _)| ***parent == city_entity)
            .flat_map(|(_, segment)| segment.points())
            .find(|vertex| vertex.distance(point) < info.half_width)
            .unwrap_or(point);

        info!("spawning new road");
        commands.entity(city_entity).with_children(|parent| {
            parent.spawn(PlacingRoadBundle::new(
                PlacingRoad::Spawning(placing_id.0),
                info.half_width,
                Segment::splat(snapped_point),
                PlacingSegment {
                    point_kind: PointKind::End,
                    snap_offset: info.half_width,
                },
                asset_server.load(info.material.clone()),
                meshes.add(DynamicMesh::create_empty()),
            ));
        });
    }

    fn update_alpha(
        mut placing_roads: Query<
            (&mut AlphaColor, &CollidingEntities),
            (Changed<CollidingEntities>, With<PlacingRoad>),
        >,
    ) {
        let Ok((mut alpha, colliding_entities)) = placing_roads.get_single_mut() else {
            return;
        };

        if colliding_entities.is_empty() {
            **alpha = WHITE.into();
        } else {
            **alpha = RED.into();
        };
    }

    fn delete(
        _trigger: Trigger<Completed<DeleteSegment>>,
        city_mode: Option<Res<State<CityMode>>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_roads: Query<(Entity, &PlacingRoad, &mut Segment)>,
        roads: Query<&Segment, Without<PlacingRoad>>,
    ) {
        if !observer_in_state(city_mode, CityMode::Roads) {
            return;
        }

        let Ok((placing_entity, &placing_road, mut segment)) = placing_roads.get_single_mut()
        else {
            return;
        };

        info!("deleting road");
        if let PlacingRoad::EditPoint { entity } = placing_road {
            // Set original segment until the deletion is confirmed.
            *segment = *roads.get(entity).expect("moving road should exist");

            let command_id = history.push_pending(RoadCommand::Delete { entity });
            commands
                .entity(placing_entity)
                .insert(PendingDespawn { command_id })
                .remove::<PlacingRoad>();
        } else {
            commands.entity(placing_entity).despawn_recursive();
        }
    }

    fn cancel(
        _trigger: Trigger<Completed<CancelSegment>>,
        city_mode: Option<Res<State<CityMode>>>,
        mut commands: Commands,
        placing_roads: Query<Entity, With<PlacingRoad>>,
    ) {
        if !observer_in_state(city_mode, CityMode::Roads) {
            return;
        }

        if let Ok(entity) = placing_roads.get_single() {
            debug!("cancelling placing");
            commands.entity(entity).despawn();
        }
    }

    fn confirm(
        _trigger: Trigger<Completed<ConfirmSegment>>,
        city_mode: Option<Res<State<CityMode>>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        asset_server: Res<AssetServer>,
        mut placing_roads: Query<(Entity, &Parent, &Segment, &PlacingRoad, &PlacingSegment)>,
    ) {
        if !observer_in_state(city_mode, CityMode::Roads) {
            return;
        }

        let Ok((entity, parent, &segment, &placing_road, placing_segment)) =
            placing_roads.get_single_mut()
        else {
            return;
        };

        info!("configrming {placing_road:?}");
        let command_id = match placing_road {
            PlacingRoad::Spawning(id) => {
                let info_path = asset_server
                    .get_path(id)
                    .expect("info should always come from file");
                history.push_pending(RoadCommand::Create {
                    city_entity: **parent,
                    info_path: info_path.into_owned(),
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
            .entity(entity)
            .insert(PendingDespawn { command_id })
            .remove::<PlacingRoad>();
    }
}

/// ID to spawn new roads with.
///
/// Spawning won't start until this resource is inserted.
#[derive(Resource)]
pub struct SpawnRoadId(pub AssetId<RoadInfo>);

#[derive(Bundle)]
struct PlacingRoadBundle {
    name: Name,
    placing_road: PlacingRoad,
    road_data: RoadData,
    segment: Segment,
    placing_segment: PlacingSegment,
    // Looks like AABB is not recalculated when we edit the mesh.
    // But we don't need to cull currently placed road anyway.
    no_culling: NoFrustumCulling,
    state_scoped: StateScoped<RoadTool>,
    alpha: AlphaColor,
    collider: Collider,
    collision_layers: CollisionLayers,
    pbr_bundle: PbrBundle,
}

impl PlacingRoadBundle {
    fn new(
        placing_road: PlacingRoad,
        half_width: f32,
        segment: Segment,
        placing_segment: PlacingSegment,
        material: Handle<StandardMaterial>,
        mesh: Handle<Mesh>,
    ) -> Self {
        let tool = match placing_road {
            PlacingRoad::Spawning(_) => RoadTool::Create,
            PlacingRoad::EditPoint { .. } => RoadTool::Move,
        };
        Self {
            name: Name::new("Placing road"),
            road_data: RoadData { half_width },
            placing_road,
            segment,
            placing_segment,
            no_culling: NoFrustumCulling,
            state_scoped: StateScoped(tool),
            alpha: AlphaColor(WHITE.into()),
            collider: Default::default(),
            collision_layers: CollisionLayers::new(
                Layer::PlacingRoad,
                [Layer::Wall, Layer::PlacingWall],
            ),
            pbr_bundle: PbrBundle {
                material,
                mesh,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
enum PlacingRoad {
    Spawning(AssetId<RoadInfo>),
    EditPoint { entity: Entity },
}
