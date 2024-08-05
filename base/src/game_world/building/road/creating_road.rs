use bevy::{
    color::palettes::css::{RED, WHITE},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{Road, RoadCreate, RoadCreateConfirmed};
use crate::{
    asset::info::road_info::RoadInfo,
    game_world::{
        building::{lot::LotVertices, spline::SplineSegment},
        city::{ActiveCity, CityMode},
        player_camera::CameraCaster,
    },
    math::segment::Segment,
    settings::Action,
};

pub(super) struct CreatingRoadPlugin;

impl Plugin for CreatingRoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            Self::end_creation
                .after(ClientSet::Receive)
                .run_if(in_state(CityMode::Roads))
                .run_if(on_event::<RoadCreateConfirmed>()),
        )
        .add_systems(
            Update,
            (
                (
                    Self::start_creation
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<CreatingRoad>)),
                    Self::update_end,
                )
                    .run_if(resource_exists::<CreatingRoadId>),
                Self::update_material,
                Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                Self::end_creation.run_if(action_just_pressed(Action::Cancel)),
            )
                .run_if(in_state(CityMode::Roads)),
        );
    }
}

impl CreatingRoadPlugin {
    fn start_creation(
        camera_caster: CameraCaster,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        roads_info: Res<Assets<RoadInfo>>,
        creating_id: Res<CreatingRoadId>,
        roads: Query<&SplineSegment, With<Road>>,
        lots: Query<&LotVertices>,
        cities: Query<(Entity, &Children), With<ActiveCity>>,
    ) {
        if let Some(point) = camera_caster.intersect_ground().map(|point| point.xz()) {
            if !lots.iter().any(|vertices| vertices.contains_point(point)) {
                let (city_entity, children) = cities.single();
                let info = roads_info
                    .get(creating_id.0)
                    .expect("info should be preloaded");

                // Use an existing point if it is within the half width distance.
                let point = roads
                    .iter_many(children)
                    .flat_map(|segment| segment.points())
                    .find(|vertex| vertex.distance(point) < info.half_width)
                    .unwrap_or(point);

                info!("spawning new road");
                let info_path = asset_server
                    .get_path(creating_id.0)
                    .expect("info should always come from file");
                commands.entity(city_entity).with_children(|parent| {
                    parent.spawn((
                        StateScoped(CityMode::Roads),
                        CreatingRoad,
                        Road(info_path.into_owned()),
                        SplineSegment(Segment::splat(point)),
                    ));
                });
            }
        }
    }

    fn update_material(
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut roads: Query<
            (&mut Handle<StandardMaterial>, &CollidingEntities),
            (
                Changed<CollidingEntities>,
                With<CreatingRoad>,
                Without<UnconfirmedRoad>,
            ),
        >,
    ) {
        if let Ok((mut material_handle, colliding_entities)) = roads.get_single_mut() {
            let mut material = materials
                .get(&*material_handle)
                .cloned()
                .expect("material handle should be valid");

            material.alpha_mode = AlphaMode::Add;
            material.base_color = if colliding_entities.is_empty() {
                WHITE.into()
            } else {
                RED.into()
            };
            debug!("setting base color to `{:?}`", material.base_color);

            *material_handle = materials.add(material);
        }
    }

    fn update_end(
        camera_caster: CameraCaster,
        roads_info: Res<Assets<RoadInfo>>,
        creating_id: Res<CreatingRoadId>,
        mut creating_roads: Query<
            (&mut SplineSegment, &Parent),
            (With<CreatingRoad>, Without<UnconfirmedRoad>),
        >,
        roads: Query<&SplineSegment, (With<Road>, Without<CreatingRoad>)>,
        children: Query<&Children>,
    ) {
        if let Ok((mut segment, parent)) = creating_roads.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground().map(|pos| pos.xz()) {
                let info = roads_info
                    .get(creating_id.0)
                    .expect("info should be preloaded");

                // Use an already existing vertex if it is within the half width distance if one exists.
                let vertex = roads
                    .iter_many(children.get(**parent).into_iter().flatten())
                    .flat_map(|segment| segment.points())
                    .find(|vertex| vertex.distance(point) < info.half_width)
                    .unwrap_or(point);

                trace!("updating road end to `{vertex:?}`");
                segment.end = vertex;
            }
        }
    }

    fn confirm(
        mut commands: Commands,
        mut create_events: EventWriter<RoadCreate>,
        mut roads: Query<
            (Entity, &Parent, &SplineSegment, &Road),
            (With<CreatingRoad>, Without<UnconfirmedRoad>),
        >,
    ) {
        if let Ok((road_entity, parent, &segment, road)) = roads.get_single_mut() {
            info!("configrming road");
            commands.entity(road_entity).insert(UnconfirmedRoad);

            create_events.send(RoadCreate {
                city_entity: **parent,
                info_path: road.0.clone(),
                segment,
            });
        }
    }

    fn end_creation(mut commands: Commands, roads: Query<Entity, With<CreatingRoad>>) {
        if let Ok(entity) = roads.get_single() {
            debug!("ending road creation");
            commands.entity(entity).despawn();
        }
    }
}

/// Resource with selected road ID.
///
/// Creation won't start until this resource is inserted.
#[derive(Resource)]
pub struct CreatingRoadId(pub AssetId<RoadInfo>);

#[derive(Component, Default)]
pub struct CreatingRoad;

#[derive(Component)]
pub(crate) struct UnconfirmedRoad;
