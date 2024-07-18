use bevy::{
    color::palettes::css::{RED, WHITE},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{Wall, WallCreate, WallCreateConfirmed};
use crate::{
    core::GameState,
    game_world::{
        building::lot::LotVertices,
        family::{BuildingMode, FamilyMode},
        player_camera::CameraCaster,
    },
    math::segment::Segment,
    settings::Action,
};

pub(super) struct CreatingWallPlugin;

impl Plugin for CreatingWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(FamilyMode::Building), Self::end_creating)
            .add_systems(OnExit(BuildingMode::Walls), Self::end_creating)
            .add_systems(
                PreUpdate,
                Self::end_creating
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::Family))
                    .run_if(in_state(FamilyMode::Building))
                    .run_if(in_state(BuildingMode::Walls))
                    .run_if(on_event::<WallCreateConfirmed>()),
            )
            .add_systems(
                Update,
                (
                    Self::start_creating
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<CreatingWall>)),
                    Self::update_end,
                    Self::update_material,
                    Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                    Self::end_creating.run_if(action_just_pressed(Action::Cancel)),
                )
                    .run_if(in_state(GameState::Family))
                    .run_if(in_state(FamilyMode::Building))
                    .run_if(in_state(BuildingMode::Walls)),
            );
    }
}

const SNAP_DELTA: f32 = 0.5;

impl CreatingWallPlugin {
    fn start_creating(
        camera_caster: CameraCaster,
        mut commands: Commands,
        walls: Query<&Wall>,
        lots: Query<(Entity, Option<&Children>, &LotVertices)>,
    ) {
        if let Some(point) = camera_caster.intersect_ground().map(|point| point.xz()) {
            if let Some((entity, children, _)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(point))
            {
                // Use an existing point if it is within the `SNAP_DELTA` distance.
                let point = walls
                    .iter_many(children.into_iter().flatten())
                    .flat_map(|wall| [wall.start, wall.end])
                    .find(|vertex| vertex.distance(point) < SNAP_DELTA)
                    .unwrap_or(point);

                info!("spawning new wall");
                commands.entity(entity).with_children(|parent| {
                    parent.spawn((CreatingWall, Wall(Segment::splat(point))));
                });
            }
        }
    }

    fn update_material(
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut walls: Query<
            (&mut Handle<StandardMaterial>, &CollidingEntities),
            (
                Changed<CollidingEntities>,
                With<CreatingWall>,
                Without<UnconfirmedWall>,
            ),
        >,
    ) {
        if let Ok((mut material_handle, colliding_entities)) = walls.get_single_mut() {
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
        mut creating_walls: Query<
            (&mut Wall, &Parent),
            (With<CreatingWall>, Without<UnconfirmedWall>),
        >,
        walls: Query<&Wall, Without<CreatingWall>>,
        children: Query<&Children>,
    ) {
        if let Ok((mut wall, parent)) = creating_walls.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground().map(|pos| pos.xz()) {
                let children = children.get(**parent).unwrap();

                // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
                let vertex = walls
                    .iter_many(children)
                    .flat_map(|wall| [wall.start, wall.end])
                    .find(|vertex| vertex.distance(point) < SNAP_DELTA)
                    .unwrap_or(point);

                trace!("updating wall end to `{vertex:?}`");
                wall.end = vertex;
            }
        }
    }

    fn confirm(
        mut commands: Commands,
        mut create_events: EventWriter<WallCreate>,
        mut walls: Query<(Entity, &Parent, &Wall), (With<CreatingWall>, Without<UnconfirmedWall>)>,
    ) {
        if let Ok((wall_entity, parent, &wall)) = walls.get_single_mut() {
            info!("configrming wall");
            commands.entity(wall_entity).insert(UnconfirmedWall);

            create_events.send(WallCreate {
                lot_entity: **parent,
                wall,
            });
        }
    }

    fn end_creating(mut commands: Commands, walls: Query<Entity, With<CreatingWall>>) {
        if let Ok(entity) = walls.get_single() {
            debug!("despawning confirmed wall");
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component, Default)]
pub struct CreatingWall;

#[derive(Component)]
pub(crate) struct UnconfirmedWall;
