use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::{
    action_just_pressed, action_just_released, action_pressed,
};

use super::{Wall, WallSpawn};
use crate::core::{
    action::Action,
    cursor_hover::CursorHover,
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    lot::LotVertices,
};

pub(super) struct SpawningWallPlugin;

impl Plugin for SpawningWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::spawn_system
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(not(any_with_component::<SpawningWall>)),
                Self::movement_system
                    .run_if(action_pressed(Action::Confirm))
                    .run_if(any_with_component::<SpawningWall>),
                Self::confirmation_system
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
    fn spawn_system(
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
                    parent.spawn(SpawningWallBundle::new(point));
                });
            }
        }
    }

    fn movement_system(
        mut spawning_walls: Query<(&mut Wall, &Parent), With<SpawningWall>>,
        walls: Query<&Wall, Without<SpawningWall>>,
        children: Query<&Children>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(position) = hovered.get_single().map(|hover| hover.xz()) {
            let (mut wall, parent) = spawning_walls.single_mut();
            let children = children.get(**parent).unwrap();

            // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
            let vertex = walls
                .iter_many(children)
                .flat_map(|wall| [wall.start, wall.end])
                .find(|vertex| vertex.distance(position) < SNAP_DELTA)
                .unwrap_or(position);

            wall.end = vertex;
        }
    }

    fn confirmation_system(
        mut commands: Commands,
        meshes: Res<Assets<Mesh>>,
        mut spawn_events: EventWriter<WallSpawn>,
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

        spawn_events.send(WallSpawn {
            lot_entity: **parent,
            wall_entity,
            wall,
        });
    }
}

#[derive(Bundle)]
struct SpawningWallBundle {
    wall: Wall,
    parent_sync: ParentSync,
    spawning_wall: SpawningWall,
}

impl SpawningWallBundle {
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
