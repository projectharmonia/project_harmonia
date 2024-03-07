use bevy::{math::Vec3Swizzles, prelude::*};
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::core::{
    action::Action,
    city::{ActiveCity, CityMode},
    cursor_hover::CursorHover,
    game_state::GameState,
};

use super::{LotCreate, LotEventConfirmed, LotTool, LotVertices};

pub(super) struct CreatingLotPlugin;

impl Plugin for CreatingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::start_creating
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(not(any_with_component::<CreatingLot>)),
                Self::set_vertex_position,
                Self::confirm_vertex.run_if(action_just_pressed(Action::Confirm)),
                Self::cleanup.run_if(
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
    fn start_creating(mut commands: Commands, ground: Query<(&Parent, &CursorHover)>) {
        if let Ok((parent, hover)) = ground.get_single() {
            // Spawn with two the same vertices because we edit the last one on cursor movement.
            commands.entity(**parent).with_children(|parent| {
                parent.spawn((LotVertices(vec![hover.xz(); 2]), CreatingLot));
            });
        }
    }

    fn set_vertex_position(
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

    fn confirm_vertex(
        mut create_events: EventWriter<LotCreate>,
        mut creating_lots: Query<&mut LotVertices, With<CreatingLot>>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
            let first_position = *lot_vertices
                .first()
                .expect("vertices should have at least initial position");
            let last_position = *lot_vertices.last().unwrap();
            if first_position == last_position {
                create_events.send(LotCreate {
                    vertices: lot_vertices.0.clone(),
                    city_entity: active_cities.single(),
                });
            } else {
                lot_vertices.push(last_position);
            }
        }
    }

    fn cleanup(mut commands: Commands, creating_lots: Query<Entity, With<CreatingLot>>) {
        if let Ok(entity) = creating_lots.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub(crate) struct CreatingLot;
