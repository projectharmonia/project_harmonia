use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    ground::GroundPlugin,
    lot::LotVertices,
};

use super::{WallCreate, WallEdges, WallEventConfirmed};

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
                    parent.spawn((WallEdges(vec![(position, position)]), CreatingWall));
                });
            }
        }
    }

    fn movement_system(
        In(position): In<Option<Vec2>>,
        mut creating_walls: Query<&mut WallEdges, With<CreatingWall>>,
    ) {
        if let Some(position) = position {
            let mut edge = creating_walls.single_mut();
            let mut edge = edge
                .last_mut()
                .expect("creating wall should always consist of one edge");
            edge.1 = position;
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
