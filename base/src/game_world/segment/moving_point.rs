use bevy::prelude::*;

use super::{CameraCaster, PointKind, Segment};
use crate::game_world::{city::CityMode, family::building::BuildingMode};

pub(super) struct MovingPointPlugin;

impl Plugin for MovingPointPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::update_position
                .run_if(in_state(BuildingMode::Walls).or_else(in_state(CityMode::Roads))),
        );
    }
}

impl MovingPointPlugin {
    fn update_position(
        camera_caster: CameraCaster,
        mut moving_segments: Query<(&mut Segment, &Parent, &MovingPoint)>,
        segments: Query<(&Parent, &Segment), Without<MovingPoint>>,
    ) {
        let Ok((mut segment, moving_parent, moving_point)) = moving_segments.get_single_mut()
        else {
            return;
        };

        let Some(point) = camera_caster.intersect_ground().map(|pos| pos.xz()) else {
            return;
        };

        // Use an already existing vertex if it is within the `snap_offset` distance if one exists.
        let snapped_point = segments
            .iter()
            .filter(|(parent, _)| *parent == moving_parent)
            .flat_map(|(_, segment)| segment.points())
            .find(|vertex| vertex.distance(point) < moving_point.snap_offset)
            .unwrap_or(point);

        trace!("updating `{:?}` to `{snapped_point:?}`", moving_point.kind);
        segment.set_point(moving_point.kind, snapped_point);
    }
}

#[derive(Component)]
pub(crate) struct MovingPoint {
    pub(crate) kind: PointKind,
    pub(crate) snap_offset: f32,
}
