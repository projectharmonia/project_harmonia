use bevy::{math::Vec3Swizzles, prelude::*, window::PrimaryWindow};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::{
    action_just_pressed, action_just_released, action_pressed,
};

use super::{Wall, WallCreate};
use crate::core::{
    action::Action,
    cursor_hover::CursorHover,
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    lot::LotVertices,
    player_camera::PlayerCamera,
};

pub(super) struct SpawningWallPlugin;

impl Plugin for SpawningWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::start_creating
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(not(any_with_component::<SpawningWall>)),
                Self::update_end
                    .run_if(action_pressed(Action::Confirm))
                    .run_if(any_with_component::<SpawningWall>),
                Self::confirm
                    .run_if(action_just_released(Action::Confirm))
                    .run_if(any_with_component::<SpawningWall>),
            )
                .run_if(in_state(GameState::Family))
                .run_if(in_state(FamilyMode::Building))
                .run_if(in_state(BuildingMode::Walls)),
        );
    }
}

const SNAP_DELTA: f32 = 0.5;

impl SpawningWallPlugin {
    fn start_creating(
        mut commands: Commands,
        walls: Query<&Wall>,
        lots: Query<(Entity, Option<&Children>, &LotVertices)>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(position) = hovered.get_single().map(|hover| hover.xz()) {
            if let Some((entity, children, _)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(position))
            {
                // Use an existing point if it is within the `SNAP_DELTA` distance.
                let point = walls
                    .iter_many(children.into_iter().flatten())
                    .flat_map(|wall| [wall.start, wall.end])
                    .find(|vertex| vertex.distance(position) < SNAP_DELTA)
                    .unwrap_or(position);

                commands.entity(entity).with_children(|parent| {
                    parent.spawn(CreatingWallBundle::new(point));
                });
            }
        }
    }

    fn update_end(
        mut spawning_walls: Query<(&mut Wall, &Parent), With<SpawningWall>>,
        walls: Query<&Wall, Without<SpawningWall>>,
        children: Query<&Children>,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
    ) {
        let Some(cursor_pos) = windows.single().cursor_position() else {
            return;
        };

        let (&camera_transform, camera) = cameras.single();
        let ray = camera
            .viewport_to_world(&camera_transform, cursor_pos)
            .expect("ray should be created from screen coordinates");

        let Some(distance) = ray.intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y)) else {
            return;
        };

        let (mut wall, parent) = spawning_walls.single_mut();
        let children = children.get(**parent).unwrap();

        // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
        let position = ray.get_point(distance).xz();
        let vertex = walls
            .iter_many(children)
            .flat_map(|wall| [wall.start, wall.end])
            .find(|vertex| vertex.distance(position) < SNAP_DELTA)
            .unwrap_or(position);

        wall.end = vertex;
    }

    fn confirm(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        mut create_events: EventWriter<WallCreate>,
        mut spawning_walls: Query<
            (Entity, &Parent, &Wall, &Handle<Mesh>, &mut Collider),
            With<SpawningWall>,
        >,
    ) {
        let (wall_entity, parent, &wall, mesh_handle, mut collider) = spawning_walls.single_mut();

        let mesh = meshes
            .get(mesh_handle)
            .expect("spawning wall mesh handle should be walid");
        *collider = Collider::trimesh_from_mesh(mesh)
            .expect("spawnign wall mesh should be in compatible format");

        commands
            .entity(wall_entity)
            .remove::<SpawningWall>()
            .insert(Replication);

        create_events.send(WallCreate {
            lot_entity: **parent,
            wall_entity,
            wall,
        });
    }
}

#[derive(Bundle)]
struct CreatingWallBundle {
    wall: Wall,
    parent_sync: ParentSync,
    spawning_wall: SpawningWall,
}

impl CreatingWallBundle {
    fn new(point: Vec2) -> Self {
        Self {
            wall: Wall {
                start: point,
                end: point,
            },
            parent_sync: Default::default(),
            spawning_wall: SpawningWall,
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct SpawningWall;
