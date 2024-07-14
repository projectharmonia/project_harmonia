use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use super::{PlaceState, PlacingObjectPlugin, RotationLimit};
use crate::{
    city::CityMode,
    family::FamilyMode,
    game_state::GameState,
    wall::{wall_mesh::HALF_WIDTH, Wall},
};

pub(super) struct WallSnapPlugin;

impl Plugin for WallSnapPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WallSnap>().add_systems(
            Update,
            (
                Self::init_placing,
                Self::snap
                    .before(PlacingObjectPlugin::check_collision)
                    .after(PlacingObjectPlugin::apply_position),
            )
                .chain()
                .run_if(
                    in_state(GameState::City)
                        .and_then(in_state(CityMode::Objects))
                        .or_else(
                            in_state(GameState::Family).and_then(in_state(FamilyMode::Building)),
                        ),
                ),
        );
    }
}

impl WallSnapPlugin {
    fn init_placing(mut placing_objects: Query<(&mut PlaceState, &WallSnap), Added<WallSnap>>) {
        if let Ok((mut placing_object, snap)) = placing_objects.get_single_mut() {
            if snap.required() {
                placing_object.allowed_place = false;
            }
        }
    }

    fn snap(
        walls: Query<&Wall>,
        mut placing_objects: Query<(
            &mut Position,
            &mut Rotation,
            &mut PlaceState,
            &mut RotationLimit,
            &WallSnap,
        )>,
    ) {
        let Ok((mut position, mut rotation, mut state, mut limit, snap)) =
            placing_objects.get_single_mut()
        else {
            return;
        };

        const SNAP_DELTA: f32 = 1.0;
        let object_point = position.xz();
        if let Some((wall, wall_point)) = walls
            .iter()
            .map(|wall| (wall, wall.closest_point(object_point)))
            .find(|(_, point)| point.distance(object_point) <= SNAP_DELTA)
        {
            const GAP: f32 = 0.03; // A small gap between the object and wall to avoid collision.
            let disp = wall.displacement();
            let sign = disp.perp_dot(object_point - wall_point).signum();
            let offset = match snap {
                WallSnap::Inside => Vec2::ZERO,
                WallSnap::Outside { .. } => sign * disp.perp().normalize() * (HALF_WIDTH + GAP),
            };
            let snap_point = wall_point + offset;
            let angle = disp.angle_between(Vec2::X * sign);
            position.x = snap_point.x;
            position.z = snap_point.y;
            if limit.is_none() {
                // Apply rotation only for newly snapped objects.
                **rotation = Quat::from_rotation_y(angle);
                limit.0 = Some(PI);
                if snap.required() {
                    state.allowed_place = true;
                }
            }
        } else if limit.is_some() {
            limit.0 = None;
            if snap.required() {
                state.allowed_place = false;
            }
        }
    }
}

/// Enables attaching objects to walls.
#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component)]
pub(crate) enum WallSnap {
    /// Place inside a wall, like a door or a window.
    ///
    /// Object will be required to placed inside.
    Inside,

    /// Attach to a wall, like painting.
    Outside {
        /// Requires an object to be placed on a wall.
        required: bool,
    },
}

impl WallSnap {
    fn required(self) -> bool {
        match self {
            WallSnap::Inside => true,
            WallSnap::Outside { required } => required,
        }
    }
}
