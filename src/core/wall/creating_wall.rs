use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    ground::GroundPlugin,
    lot::LotVertices,
};

use super::WallEdges;

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
        );
    }
}

impl CreatingWallPlugin {
    fn spawn_system(
        In(position): In<Option<Vec2>>,
        mut commands: Commands,
        lots: Query<(Entity, &LotVertices)>,
    ) {
        if let Some(position) = position {
            if let Some(entity) = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(position))
                .map(|(lot_entity, _)| lot_entity)
            {
                commands.entity(entity).with_children(|parent| {
                    parent.spawn((WallEdges(vec![(position, position + 10.0)]), CreatingWall));
                });
            }
        }
    }

    fn movement_system(
        In(position): In<Option<Vec2>>,
        mut creating_walls: Query<&mut WallEdges, With<CreatingWall>>,
    ) {
        if let Some(position) = position {
            let mut wall = creating_walls.single_mut();
            let mut edge = wall
                .last_mut()
                .expect("creating wall should always consist of one edge");
            edge.1 = position;
        }
    }
}

pub(crate) fn creating_active(building_walls: Query<(), With<CreatingWall>>) -> bool {
    !building_walls.is_empty()
}

#[derive(Component, Default)]
pub(crate) struct CreatingWall;
