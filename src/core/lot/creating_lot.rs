use bevy::{math::Vec3Swizzles, prelude::*};
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::core::{
    action::Action,
    city::{ActiveCity, CityMode},
    cursor_hover::CursorHover,
    game_state::GameState,
};

use super::{LotEventConfirmed, LotSpawn, LotTool, LotVertices};

pub(super) struct CreatingLotPlugin;

impl Plugin for CreatingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::spawn_system
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(not(any_with_component::<CreatingLot>)),
                Self::movement_system,
                Self::vertex_placement_system.run_if(action_just_pressed(Action::Confirm)),
                Self::despawn_system.run_if(
                    action_just_pressed(Action::Cancel).or_else(on_event::<LotEventConfirmed>()),
                ),
            )
                .run_if(in_state(GameState::City))
                .run_if(in_state(CityMode::Lots))
                .run_if(in_state(LotTool::Create)),
        );
    }
}

impl CreatingLotPlugin {
    fn spawn_system(mut commands: Commands, ground: Query<(&Parent, &CursorHover)>) {
        if let Ok((parent, hover)) = ground.get_single() {
            // Spawn with two the same vertices because we edit the last one on cursor movement.
            commands.entity(**parent).with_children(|parent| {
                parent.spawn((LotVertices(vec![hover.xz(); 2]), CreatingLot));
            });
        }
    }

    fn movement_system(
        mut creating_lots: Query<&mut LotVertices, With<CreatingLot>>,
        ground: Query<&CursorHover>,
    ) {
        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
            if let Ok(new_position) = ground.get_single().map(|hover| hover.xz()) {
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
        mut creating_lots: Query<&mut LotVertices, With<CreatingLot>>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
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

    fn despawn_system(mut commands: Commands, creating_lots: Query<Entity, With<CreatingLot>>) {
        if let Ok(entity) = creating_lots.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub(crate) struct CreatingLot;
