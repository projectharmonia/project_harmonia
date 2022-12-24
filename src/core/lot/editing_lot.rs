use bevy::{math::Vec3Swizzles, prelude::*};
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    game_state::{CursorMode, GameState},
    preview::PreviewCamera,
};

use super::{LotSpawn, LotSpawnConfirmed, LotVertices};

pub(super) struct EditingLotPlugin;

impl Plugin for EditingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::spawn_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_if_not(editing_active)
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
                .run_on_event::<LotSpawnConfirmed>()
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Lots),
        );
    }
}

impl EditingLotPlugin {
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
        commands.spawn((LotVertices(vec![position; 2]), EditingLot));
    }

    fn movement_system(
        windows: Res<Windows>,
        mut editing_lots: Query<&mut LotVertices, With<EditingLot>>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
    ) {
        if let Ok(mut lot_vertices) = editing_lots.get_single_mut() {
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
    ) {
        if let Ok(mut lot_vertices) = editing_lots.get_single_mut() {
            let first_position = *lot_vertices
                .first()
                .expect("vertices should have at least initial position");
            let last_position = *lot_vertices.last().unwrap();
            if first_position == last_position {
                spawn_events.send(LotSpawn(lot_vertices.0.clone()));
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

fn intersection_with_ground(
    cameras: &Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
    cursor_pos: Vec2,
) -> Vec2 {
    let (&transform, camera) = cameras.single();
    let ray = camera
        .viewport_to_world(&transform, cursor_pos)
        .expect("ray should be created from screen coordinates");
    let length = -ray.origin.y / ray.direction.y; // The length to intersect the plane.
    let intersection = ray.origin + ray.direction * length;
    intersection.xz() // y is always 0.
}

pub(crate) fn editing_active(spawning_lots: Query<(), With<EditingLot>>) -> bool {
    !spawning_lots.is_empty()
}

#[derive(Component)]
pub(crate) struct EditingLot;
