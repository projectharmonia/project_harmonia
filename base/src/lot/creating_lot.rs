use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_replicon::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{LotCreate, LotEventConfirmed, LotTool, LotVertices, UnconfirmedLot};
use crate::{
    city::{ActiveCity, CityMode},
    game_state::GameState,
    player_camera::CameraCaster,
    settings::Action,
};

pub(super) struct CreatingLotPlugin;

impl Plugin for CreatingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(CityMode::Lots), Self::end_creating)
            .add_systems(OnExit(LotTool::Create), Self::end_creating)
            .add_systems(
                PreUpdate,
                Self::end_creating
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::City))
                    .run_if(in_state(CityMode::Lots))
                    .run_if(in_state(LotTool::Create))
                    .run_if(on_event::<LotEventConfirmed>()),
            )
            .add_systems(
                Update,
                (
                    Self::start_creating
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<CreatingLot>)),
                    Self::set_vertex_position,
                    Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                    Self::end_creating.run_if(action_just_pressed(Action::Cancel)),
                )
                    .run_if(in_state(GameState::City))
                    .run_if(in_state(CityMode::Lots))
                    .run_if(in_state(LotTool::Create)),
            );
    }
}

impl CreatingLotPlugin {
    fn start_creating(
        camera_caster: CameraCaster,
        mut commands: Commands,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Some(point) = camera_caster.intersect_ground() {
            // Spawn with two the same vertices because we edit the last one on cursor movement.
            commands.entity(cities.single()).with_children(|parent| {
                parent.spawn((LotVertices(vec![point.xz(); 2].into()), CreatingLot));
            });
        }
    }

    fn set_vertex_position(
        camera_caster: CameraCaster,
        mut creating_lots: Query<&mut LotVertices, (With<CreatingLot>, Without<UnconfirmedLot>)>,
    ) {
        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground().map(|hover| hover.xz()) {
                let first_vertex = *lot_vertices
                    .first()
                    .expect("vertices should have at least 2 vertices");
                let last_vertex = lot_vertices.last_mut().unwrap();

                const SNAP_DELTA: f32 = 0.1;
                let delta = first_vertex - point;
                if delta.x.abs() <= SNAP_DELTA && delta.y.abs() <= SNAP_DELTA {
                    *last_vertex = first_vertex;
                } else {
                    *last_vertex = point;
                }
            }
        }
    }

    fn confirm(
        mut create_events: EventWriter<LotCreate>,
        mut creating_lots: Query<&mut LotVertices, (With<CreatingLot>, Without<UnconfirmedLot>)>,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
            let first_vertex = *lot_vertices
                .first()
                .expect("vertices should have at least 2 vertices");
            let last_vertex = *lot_vertices.last().unwrap();
            if first_vertex == last_vertex {
                create_events.send(LotCreate {
                    polygon: lot_vertices.0.clone(),
                    city_entity: cities.single(),
                });
            } else {
                lot_vertices.push(last_vertex);
            }
        }
    }

    fn end_creating(mut commands: Commands, creating_lots: Query<Entity, With<CreatingLot>>) {
        if let Ok(entity) = creating_lots.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub struct CreatingLot;
