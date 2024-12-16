use std::f32::consts::PI;

use bevy::prelude::*;

use super::{ObjectRotationLimit, PlacingObjectPlugin, PlacingObjectState};
use crate::game_world::{
    city::CityMode,
    family::building::{
        wall::{wall_mesh::HALF_WIDTH, Wall},
        BuildingMode,
    },
    segment::Segment,
};

pub(super) struct WallSnapPlugin;

impl Plugin for WallSnapPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WallSnap>().add_systems(
            Update,
            (
                Self::init_placing,
                Self::snap.after(PlacingObjectPlugin::apply_position),
            )
                .chain()
                .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
        );
    }
}

impl WallSnapPlugin {
    fn init_placing(
        mut placing_objects: Query<(&mut PlacingObjectState, &WallSnap), Added<WallSnap>>,
    ) {
        if let Ok((mut placing_object, snap)) = placing_objects.get_single_mut() {
            if snap.required() {
                debug!("disabling placing until snapped");
                placing_object.allowed_place = false;
            }
        }
    }

    fn snap(
        walls: Query<&Segment, With<Wall>>,
        mut placing_objects: Query<(
            &mut Transform,
            &mut PlacingObjectState,
            &mut ObjectRotationLimit,
            &WallSnap,
        )>,
    ) {
        let Ok((mut transform, mut state, mut rotation_limit, snap)) =
            placing_objects.get_single_mut()
        else {
            return;
        };

        const SNAP_DELTA: f32 = 1.0;
        let object_point = transform.translation.xz();
        if let Some((wall, wall_point)) = walls
            .iter()
            .map(|wall| (wall, wall.closest_point(object_point)))
            .find(|(_, point)| point.distance(object_point) <= SNAP_DELTA)
        {
            trace!("snapping to wall");
            const GAP: f32 = 0.03; // A small gap between the object and wall to avoid collision.
            let disp = wall.displacement();
            let sign = disp.perp_dot(object_point - wall_point).signum();
            let offset = match snap {
                WallSnap::Inside => Vec2::ZERO,
                WallSnap::Outside { .. } => sign * disp.perp().normalize() * (HALF_WIDTH + GAP),
            };
            let snap_point = wall_point + offset;
            let angle = disp.angle_between(Vec2::X * sign);
            transform.translation.x = snap_point.x;
            transform.translation.z = snap_point.y;
            if rotation_limit.is_none() {
                // Apply rotation only for newly snapped objects.
                debug!("applying rotation {angle}");
                transform.rotation = Quat::from_rotation_y(angle);
                **rotation_limit = Some(PI);
                if snap.required() {
                    debug!("allowing placing");
                    state.allowed_place = true;
                }
            }
        } else if rotation_limit.is_some() {
            **rotation_limit = None;
            if snap.required() {
                debug!("disallowing placing");
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
