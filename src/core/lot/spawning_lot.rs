use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    game_state::{CursorMode, GameState},
    network::network_event::client_event::ClientSendBuffer,
    preview::PreviewCamera,
};

use super::{LotConfirmed, LotSpawn, LotVertices};

pub(super) struct SpawningLotPlugin;

impl Plugin for SpawningLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::spawn_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_if_not(spawning_active)
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Lots),
        )
        .add_system(
            Self::movement_system
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Lots),
        )
        .add_system(
            Self::vertex_placement_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Lots),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Lots),
        )
        .add_system(
            Self::despawn_system
                .run_on_event::<LotConfirmed>()
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Lots),
        );
    }
}

impl SpawningLotPlugin {
    fn spawn_system(
        mut commands: Commands,
        windows: Res<Windows>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
    ) {
        let cursor_pos = windows
            .get_primary()
            .and_then(|window| window.cursor_position())
            .unwrap_or_default();
        // Spawn with two the same vertices because we edit the last one on cursor movement.
        let position = intersection_with_ground(&cameras, cursor_pos);
        commands.spawn((LotVertices(vec![position; 2]), SpawningLot));
    }

    fn movement_system(
        windows: Res<Windows>,
        mut spawning_lots: Query<&mut LotVertices, With<SpawningLot>>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
    ) {
        if let Ok(mut lot_vertices) = spawning_lots.get_single_mut() {
            if let Some(cursor_pos) = windows
                .get_primary()
                .and_then(|window| window.cursor_position())
            {
                let new_position = intersection_with_ground(&cameras, cursor_pos);
                let first_position = *lot_vertices
                    .first()
                    .expect("vertices should have at least initial position");
                let last_position = lot_vertices.last_mut().unwrap();

                const SNAP_DELTA: f32 = 0.1;
                let delta = first_position - new_position;
                if delta.x.abs() <= SNAP_DELTA && delta.z.abs() <= SNAP_DELTA {
                    *last_position = first_position;
                } else {
                    *last_position = new_position;
                }
            }
        }
    }

    fn vertex_placement_system(
        mut spawn_events: ResMut<ClientSendBuffer<LotSpawn>>,
        mut spawning_lots: Query<&mut LotVertices, With<SpawningLot>>,
    ) {
        if let Ok(mut lot_vertices) = spawning_lots.get_single_mut() {
            let first_position = *lot_vertices
                .first()
                .expect("vertices should have at least initial position");
            let last_position = *lot_vertices.last().unwrap();
            if first_position == last_position {
                spawn_events.push(LotSpawn(lot_vertices.0.clone()));
            } else {
                lot_vertices.push(last_position);
            }
        }
    }

    fn despawn_system(mut commands: Commands, spawning_lots: Query<Entity, With<SpawningLot>>) {
        if let Ok(entity) = spawning_lots.get_single() {
            commands.entity(entity).despawn();
        }
    }
}

fn intersection_with_ground(
    cameras: &Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
    cursor_pos: Vec2,
) -> Vec3 {
    let (&transform, camera) = cameras.single();
    let ray = camera
        .viewport_to_world(&transform, cursor_pos)
        .expect("ray should be created from screen coordinates");
    let length = -ray.origin.y / ray.direction.y; // The length to intersect the plane.
    ray.origin + ray.direction * length
}

pub(crate) fn spawning_active(spawning_lots: Query<(), With<SpawningLot>>) -> bool {
    !spawning_lots.is_empty()
}

#[derive(Component)]
pub(crate) struct SpawningLot;
