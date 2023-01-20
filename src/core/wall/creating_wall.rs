use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{WallCreate, WallEdges, WallEventConfirmed};
use crate::core::{
    action::{self, Action},
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    ground::GroundPlugin,
    lot::LotVertices,
};

pub(super) struct CreatingWallPlugin;

impl Plugin for CreatingWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::spawn_system)
                .run_if(action::just_pressed(Action::Confirm))
                .run_if_not(creating_active)
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .run_in_state(BuildingMode::Walls),
        )
        .add_system(
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::movement_system)
                .run_if(action::pressed(Action::Confirm))
                .run_if(creating_active)
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .run_in_state(BuildingMode::Walls),
        )
        .add_system(
            Self::creation_system
                .run_if(action::just_released(Action::Confirm))
                .run_if(creating_active)
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .run_in_state(BuildingMode::Walls),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .run_in_state(BuildingMode::Walls),
        )
        .add_system(
            Self::despawn_system
                .run_on_event::<WallEventConfirmed>()
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .run_in_state(BuildingMode::Walls),
        );
    }
}

impl CreatingWallPlugin {
    const SNAP_DELTA: f32 = 0.5;

    fn spawn_system(
        In(position): In<Option<Vec2>>,
        mut commands: Commands,
        walls: Query<&WallEdges>,
        lots: Query<(Entity, Option<&Children>, &LotVertices)>,
    ) {
        if let Some(position) = position {
            if let Some((entity, children)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(position))
                .map(|(entity, children, _)| (entity, children))
            {
                // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
                let vertex = walls
                    .iter_many(children.iter().flat_map(|children| children.iter()))
                    .flat_map(|edges| edges.iter())
                    .flat_map(|edge| [edge.0, edge.1])
                    .find(|vertex| vertex.distance(position) < Self::SNAP_DELTA)
                    .unwrap_or(position);

                commands.entity(entity).with_children(|parent| {
                    parent.spawn((WallEdges(vec![(vertex, vertex)]), CreatingWall));
                });
            }
        }
    }

    fn movement_system(
        In(position): In<Option<Vec2>>,
        mut creating_walls: Query<(&mut WallEdges, &Parent), With<CreatingWall>>,
        walls: Query<&WallEdges, Without<CreatingWall>>,
        children: Query<&Children>,
    ) {
        if let Some(position) = position {
            let (mut edges, parent) = creating_walls.single_mut();
            let children = children.get(parent.get()).unwrap();
            let mut edge = edges
                .last_mut()
                .expect("creating wall should always consist of one edge");

            // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
            let vertex = walls
                .iter_many(children)
                .flat_map(|edges| edges.iter())
                .flat_map(|edge| [edge.0, edge.1])
                .find(|vertex| vertex.distance(position) < Self::SNAP_DELTA)
                .unwrap_or(position);

            edge.1 = vertex;
        }
    }

    fn creation_system(
        mut create_events: EventWriter<WallCreate>,
        creating_walls: Query<(&Parent, &WallEdges), With<CreatingWall>>,
    ) {
        let (parent, edges) = creating_walls.single();
        let edge = *edges
            .last()
            .expect("creating wall should always consist of one edge");
        create_events.send(WallCreate {
            lot_entity: parent.get(),
            edge,
        });
    }

    fn despawn_system(mut commands: Commands, creating_walls: Query<Entity, With<CreatingWall>>) {
        if let Ok(entity) = creating_walls.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn creating_active(building_walls: Query<(), With<CreatingWall>>) -> bool {
    !building_walls.is_empty()
}

#[derive(Component, Default)]
pub(crate) struct CreatingWall;
