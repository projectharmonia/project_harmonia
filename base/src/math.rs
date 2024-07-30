pub(super) mod polygon;
pub(super) mod segment;
pub(super) mod triangulator;

use bevy::prelude::*;

use polygon::Polygon;
use segment::Segment;

pub(super) struct MathPlugin;

impl Plugin for MathPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Polygon>().register_type::<Segment>();
    }
}

/// Returns [`Quat`] looking at specific direction.
///
/// Assumes that [`Vec3::Y`] is up direction.
/// If `direction` is zero, [`Vec3::NEG_Z`] is used instead.
///
/// # Panics
///
/// Panics if if `direction` is parallel with [`Vec3::Y`].
pub(crate) fn looking_to(direction: Vec3) -> Quat {
    let back = -direction.try_normalize().unwrap_or(Vec3::NEG_Z);
    let right = Vec3::Y.cross(back).normalize();

    Quat::from_mat3(&Mat3::from_cols(right, Vec3::Y, back))
}
