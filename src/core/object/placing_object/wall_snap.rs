use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use super::{PlacingObjectPlugin, PlacingObjectState};
use crate::core::{
    city::CityMode,
    family::FamilyMode,
    game_state::GameState,
    object::{ObjectComponent, ReflectObjectComponent},
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
    fn init_placing(
        mut placing_objects: Query<(&mut PlacingObjectState, &WallSnap), Added<WallSnap>>,
    ) {
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
            &mut PlacingObjectState,
            &WallSnap,
        )>,
    ) {
        let Ok((mut position, mut rotation, mut state, snap)) = placing_objects.get_single_mut()
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
            if !state.snapped {
                // Apply rotation only for newly snapped objects.
                **rotation = Quat::from_rotation_y(angle);
                state.snapped = true;
                if snap.required() {
                    state.allowed_place = true;
                }
            }
        } else if state.snapped {
            state.snapped = false;
            if snap.required() {
                state.allowed_place = false;
            }
        }
    }
}

#[derive(Component, Reflect, Clone, Copy)]
#[reflect(Component, ObjectComponent)]
pub(crate) enum WallSnap {
    Inside,
    Outside { required: bool },
}

impl WallSnap {
    fn required(self) -> bool {
        match self {
            WallSnap::Inside => true,
            WallSnap::Outside { required } => required,
        }
    }
}

impl ObjectComponent for WallSnap {
    fn insert_on_spawning(&self) -> bool {
        false
    }

    fn insert_on_placing(&self) -> bool {
        true
    }
}
