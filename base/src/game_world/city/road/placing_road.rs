use avian3d::prelude::*;
use bevy::{
    color::palettes::css::{RED, WHITE},
    math::Vec3Swizzles,
    prelude::*,
    render::view::NoFrustumCulling,
};
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{Road, RoadData, RoadTool};
use crate::{
    asset::info::road_info::RoadInfo,
    game_world::{
        city::{road::RoadCommand, ActiveCity, CityMode},
        commands_history::{CommandsHistory, PendingDespawn},
        hover::{HoverPlugin, Hovered},
        player_camera::CameraCaster,
        spline::{dynamic_mesh::DynamicMesh, PointKind, SplineSegment},
        Layer,
    },
    ghost::Ghost,
    math::segment::Segment,
    settings::Action,
};

pub(super) struct PlacingRoadPlugin;

impl Plugin for PlacingRoadPlugin {
    fn build(&self, app: &mut App) {
        app.observe(HoverPlugin::enable_on_remove::<PlacingRoad>)
            .observe(HoverPlugin::disable_on_add::<PlacingRoad>)
            .add_systems(
                Update,
                (
                    (
                        Self::spawn
                            .run_if(resource_exists::<SpawnRoadId>)
                            .run_if(in_state(RoadTool::Create)),
                        Self::pick.run_if(in_state(RoadTool::Move)),
                    )
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<PlacingRoad>)),
                    (
                        Self::update_end,
                        Self::update_material,
                        Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                        Self::delete.run_if(action_just_pressed(Action::Delete)),
                        Self::cancel.run_if(action_just_pressed(Action::Cancel)),
                    )
                        .run_if(in_state(CityMode::Roads)),
                ),
            );
    }
}

impl PlacingRoadPlugin {
    fn pick(
        mut commands: Commands,
        roads_info: Res<Assets<RoadInfo>>,
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        roads: Query<(
            Entity,
            &Parent,
            &Handle<StandardMaterial>,
            &Road,
            &SplineSegment,
            &Hovered,
        )>,
    ) {
        let Ok((entity, parent, material, road, &segment, hovered)) = roads.get_single() else {
            return;
        };

        let info_handle = asset_server
            .get_handle(&road.0)
            .expect("info should be preloaded");
        let info = roads_info.get(&info_handle).unwrap();

        let point = hovered.xz();
        let kind = if segment.start.distance(point) < info.half_width {
            PointKind::Start
        } else if segment.end.distance(point) < info.half_width {
            PointKind::End
        } else {
            return;
        };

        info!("picking `{kind:?}` for `{entity}`");
        commands.entity(**parent).with_children(|parent| {
            parent.spawn((
                Ghost::new(entity),
                PlacingRoadBundle::new(
                    PlacingRoad::MovingPoint { entity, kind },
                    info.half_width,
                    *segment,
                    material.clone(),
                    meshes.add(DynamicMesh::create_empty()),
                ),
            ));
        });
    }

    fn spawn(
        camera_caster: CameraCaster,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        asset_server: Res<AssetServer>,
        roads_info: Res<Assets<RoadInfo>>,
        placing_id: Res<SpawnRoadId>,
        roads: Query<(&Parent, &SplineSegment), With<Road>>,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        let Some(point) = camera_caster.intersect_ground().map(|point| point.xz()) else {
            return;
        };

        let city_entity = cities.single();
        let info = roads_info
            .get(placing_id.0)
            .expect("info should be preloaded");

        // Use an existing point if it is within the half width distance.
        let point = roads
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
                Segment::splat(point),
                asset_server.load(info.material.clone()),
                meshes.add(DynamicMesh::create_empty()),
            ));
        });
    }

    fn update_material(
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut placing_roads: Query<
            (&mut Handle<StandardMaterial>, &CollidingEntities),
            (Changed<CollidingEntities>, With<PlacingRoad>),
        >,
    ) {
        let Ok((mut material_handle, colliding_entities)) = placing_roads.get_single_mut() else {
            return;
        };

        let mut material = materials
            .get(&*material_handle)
            .cloned()
            .expect("material handle should be valid");

        let color = if colliding_entities.is_empty() {
            WHITE.into()
        } else {
            RED.into()
        };
        debug!("changing base color to `{color:?}`");

        material.alpha_mode = AlphaMode::Add;
        material.base_color = color;

        *material_handle = materials.add(material);
    }

    fn update_end(
        camera_caster: CameraCaster,
        mut placing_roads: Query<(&mut SplineSegment, &Parent, &PlacingRoad, &RoadData)>,
        roads: Query<(&Parent, &SplineSegment), (With<Road>, Without<PlacingRoad>)>,
    ) {
        let Ok((mut segment, placing_parent, placing_road, road_data)) =
            placing_roads.get_single_mut()
        else {
            return;
        };

        let Some(point) = camera_caster.intersect_ground().map(|pos| pos.xz()) else {
            return;
        };

        // Use an already existing vertex if it is within the half width distance if one exists.
        let vertex = roads
            .iter()
            .filter(|(parent, _)| *parent == placing_parent)
            .flat_map(|(_, segment)| segment.points())
            .find(|vertex| vertex.distance(point) < road_data.half_width)
            .unwrap_or(point);

        let point_kind = placing_road.point_kind();

        trace!("updating `{point_kind:?}` to `{vertex:?}`");
        match point_kind {
            PointKind::Start => segment.start = vertex,
            PointKind::End => segment.end = vertex,
        }
    }

    fn confirm(
        mut commands: Commands,
        mut history: CommandsHistory,
        asset_server: Res<AssetServer>,
        mut placing_roads: Query<(Entity, &Parent, &SplineSegment, &PlacingRoad)>,
    ) {
        let Ok((entity, parent, &segment, &placing_road)) = placing_roads.get_single_mut() else {
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
                    segment: *segment,
                })
            }
            PlacingRoad::MovingPoint { entity, kind } => {
                let point = match kind {
                    PointKind::Start => segment.start,
                    PointKind::End => segment.end,
                };
                history.push_pending(RoadCommand::MovePoint {
                    entity,
                    kind,
                    point,
                })
            }
        };

        commands
            .entity(entity)
            .insert(PendingDespawn { command_id })
            .remove::<PlacingRoad>();
    }

    fn delete(
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_roads: Query<(Entity, &PlacingRoad, &mut SplineSegment)>,
        roads: Query<&SplineSegment, Without<PlacingRoad>>,
    ) {
        let Ok((placing_entity, &placing_road, mut segment)) = placing_roads.get_single_mut()
        else {
            return;
        };

        info!("deleting road");
        if let PlacingRoad::MovingPoint { entity, .. } = placing_road {
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

    fn cancel(mut commands: Commands, placing_roads: Query<Entity, With<PlacingRoad>>) {
        if let Ok(entity) = placing_roads.get_single() {
            debug!("cancelling placing");
            commands.entity(entity).despawn();
        }
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
    segment: SplineSegment,
    state_scoped: StateScoped<RoadTool>,
    collider: Collider,
    collision_layers: CollisionLayers,
    no_culling: NoFrustumCulling,
    pbr_bundle: PbrBundle,
}

impl PlacingRoadBundle {
    fn new(
        placing_road: PlacingRoad,
        half_width: f32,
        segment: Segment,
        material: Handle<StandardMaterial>,
        mesh: Handle<Mesh>,
    ) -> Self {
        let tool = match placing_road {
            PlacingRoad::Spawning(_) => RoadTool::Create,
            PlacingRoad::MovingPoint { .. } => RoadTool::Move,
        };
        Self {
            name: Name::new("Placing road"),
            road_data: RoadData { half_width },
            placing_road,
            segment: SplineSegment(segment),
            state_scoped: StateScoped(tool),
            collider: Default::default(),
            collision_layers: CollisionLayers::new(
                Layer::PlacingRoad,
                [Layer::Wall, Layer::PlacingWall],
            ),
            no_culling: NoFrustumCulling,
            pbr_bundle: PbrBundle {
                material,
                mesh,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub enum PlacingRoad {
    Spawning(AssetId<RoadInfo>),
    MovingPoint { entity: Entity, kind: PointKind },
}

impl PlacingRoad {
    /// Returns point kind that should be edited for this road.
    fn point_kind(self) -> PointKind {
        match self {
            PlacingRoad::Spawning(_) => PointKind::End,
            PlacingRoad::MovingPoint { entity: _, kind } => kind,
        }
    }
}
