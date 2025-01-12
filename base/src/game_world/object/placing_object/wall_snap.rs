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
        app.register_type::<WallSnap>()
            .add_observer(Self::init_placing)
            .add_systems(
                Update,
                Self::snap
                    .never_param_warn()
                    .after(PlacingObjectPlugin::apply_position)
                    .run_if(in_state(CityMode::Objects).or(in_state(BuildingMode::Objects))),
            );
    }
}

impl WallSnapPlugin {
    fn init_placing(
        trigger: Trigger<OnAdd, WallSnap>,
        mut placing_objects: Query<(&mut PlacingObjectState, &WallSnap), Added<WallSnap>>,
    ) {
        let (mut placing_object, snap) = placing_objects.get_mut(trigger.entity()).unwrap();
        if snap.required() {
            debug!("disabling placing until snapped");
            placing_object.allowed_place = false;
        }
    }

    fn snap(
        placing_object: Single<
            (
                &mut Transform,
                &mut PlacingObjectState,
                &mut ObjectRotationLimit,
                &WallSnap,
            ),
            Without<Wall>,
        >,
        walls: Query<(&Segment, &Transform), With<Wall>>,
    ) {
        const SNAP_DELTA: f32 = 1.0;
        let (mut object_transform, mut state, mut rotation_limit, snap) =
            placing_object.into_inner();
        let object_point = object_transform.translation.xz();
        if let Some((wall, wall_transform, wall_point)) = walls
            .iter()
            .map(|(wall, transform)| (wall, transform, wall.closest_point(object_point)))
            .find(|(.., point)| point.distance(object_point) <= SNAP_DELTA)
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
            object_transform.translation.x = snap_point.x;
            object_transform.translation.z = snap_point.y;
            if rotation_limit.is_none() {
                // Apply rotation only for newly snapped objects.
                debug!("applying rotation");
                object_transform.rotation = wall_transform.rotation;
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
