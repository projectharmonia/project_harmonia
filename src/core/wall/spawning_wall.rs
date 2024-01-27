use bevy::{math::Vec3Swizzles, prelude::*};
use leafwing_input_manager::common_conditions::{
    action_just_pressed, action_just_released, action_pressed,
};

use super::{WallEdges, WallEventConfirmed, WallSpawn};
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
                    .run_if(not(any_with_component::<SpawningWall>())),
                Self::movement_system
                    .run_if(action_pressed(Action::Confirm))
                    .run_if(any_with_component::<SpawningWall>()),
                Self::confirm_system
                    .run_if(action_just_released(Action::Confirm))
                    .run_if(any_with_component::<SpawningWall>()),
                Self::despawn_system.run_if(
                    action_just_pressed(Action::Cancel).or_else(on_event::<WallEventConfirmed>()),
                ),
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
        walls: Query<&WallEdges>,
        lots: Query<(Entity, Option<&Children>, &LotVertices)>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(position) = hovered.get_single().map(|hover| hover.xz()) {
            if let Some((entity, children, _)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(position))
            {
                // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
                let vertex = walls
                    .iter_many(children.into_iter().flatten())
                    .flat_map(|edges| edges.iter())
                    .flat_map(|edge| [edge.0, edge.1])
                    .find(|vertex| vertex.distance(position) < SNAP_DELTA)
                    .unwrap_or(position);

                commands.entity(entity).with_children(|parent| {
                    parent.spawn((WallEdges(vec![(vertex, vertex)]), SpawningWall));
                });
            }
        }
    }

    fn movement_system(
        mut spawning_walls: Query<(&mut WallEdges, &Parent), With<SpawningWall>>,
        walls: Query<&WallEdges, Without<SpawningWall>>,
        children: Query<&Children>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(position) = hovered.get_single().map(|hover| hover.xz()) {
            let (mut edges, parent) = spawning_walls.single_mut();
            let children = children.get(**parent).unwrap();
            let edge = edges
                .last_mut()
                .expect("spawning wall should always consist of one edge");

            // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
            let vertex = walls
                .iter_many(children)
                .flat_map(|edges| edges.iter())
                .flat_map(|edge| [edge.0, edge.1])
                .find(|vertex| vertex.distance(position) < SNAP_DELTA)
                .unwrap_or(position);

            edge.1 = vertex;
        }
    }

    fn confirm_system(
        mut spawn_events: EventWriter<WallSpawn>,
        spawning_walls: Query<(&Parent, &WallEdges), With<SpawningWall>>,
    ) {
        let (parent, edges) = spawning_walls.single();
        let edge = *edges
            .last()
            .expect("spawning wall should always consist of one edge");
        spawn_events.send(WallSpawn {
            lot_entity: **parent,
            edge,
        });
    }

    fn despawn_system(mut commands: Commands, spawning_walls: Query<Entity, With<SpawningWall>>) {
        if let Ok(entity) = spawning_walls.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct SpawningWall;
