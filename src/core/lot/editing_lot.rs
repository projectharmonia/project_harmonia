use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    city::{ActiveCity, CityMode},
    game_state::GameState,
    ground::GroundPlugin,
};

use super::{LotEventConfirmed, LotSpawn, LotTool, LotVertices};

pub(super) struct EditingLotPlugin;

impl Plugin for EditingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::spawn_system)
                .run_if(action::just_pressed(Action::Confirm))
                .run_if_not(editing_active)
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Edit),
        )
        .add_system(
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::movement_system)
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Edit),
        )
        .add_system(
            Self::vertex_placement_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Edit),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Edit),
        )
        .add_system(
            Self::despawn_system
                .run_on_event::<LotEventConfirmed>()
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Edit),
        );
    }
}

impl EditingLotPlugin {
    fn spawn_system(
        In(position): In<Option<Vec2>>,
        mut commands: Commands,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Some(position) = position {
            // Spawn with two the same vertices because we edit the last one on cursor movement.
            commands
                .entity(active_cities.single())
                .with_children(|parent| {
                    parent.spawn((LotVertices(vec![position; 2]), EditingLot));
                });
        }
    }

    fn movement_system(
        In(new_position): In<Option<Vec2>>,
        mut editing_lots: Query<&mut LotVertices, With<EditingLot>>,
    ) {
        if let Ok(mut lot_vertices) = editing_lots.get_single_mut() {
            if let Some(new_position) = new_position {
                let first_position = *lot_vertices
                    .first()
                    .expect("vertices should have at least initial position");
                let last_position = lot_vertices.last_mut().unwrap();

                const SNAP_DELTA: f32 = 0.1;
                let delta = first_position - new_position;
                if delta.x.abs() <= SNAP_DELTA && delta.y.abs() <= SNAP_DELTA {
                    *last_position = first_position;
                } else {
                    *last_position = new_position;
                }
            }
        }
    }

    fn vertex_placement_system(
        mut spawn_events: EventWriter<LotSpawn>,
        mut editing_lots: Query<&mut LotVertices, With<EditingLot>>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Ok(mut lot_vertices) = editing_lots.get_single_mut() {
            let first_position = *lot_vertices
                .first()
                .expect("vertices should have at least initial position");
            let last_position = *lot_vertices.last().unwrap();
            if first_position == last_position {
                spawn_events.send(LotSpawn {
                    vertices: lot_vertices.0.clone(),
                    city_entity: active_cities.single(),
                });
            } else {
                lot_vertices.push(last_position);
            }
        }
    }

    fn despawn_system(mut commands: Commands, editing_lots: Query<Entity, With<EditingLot>>) {
        if let Ok(entity) = editing_lots.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn editing_active(spawning_lots: Query<(), With<EditingLot>>) -> bool {
    !spawning_lots.is_empty()
}

#[derive(Component)]
pub(crate) struct EditingLot;
