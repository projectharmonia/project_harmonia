use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::{CameraCaster, PointKind, Segment};
use crate::{
    game_world::{city::CityMode, family::building::BuildingMode},
    settings::Settings,
};

pub(super) struct PlacingSegmentPlugin;

impl Plugin for PlacingSegmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<PlacingSegment>().add_systems(
            Update,
            Self::update_position
                .run_if(in_state(BuildingMode::Walls).or_else(in_state(CityMode::Roads))),
        );
    }
}

impl PlacingSegmentPlugin {
    fn update_position(
        camera_caster: CameraCaster,
        mut placing_segments: Query<(&mut Segment, &Parent, &PlacingSegment)>,
        segments: Query<(&Parent, &Segment), Without<PlacingSegment>>,
    ) {
        let Ok((mut segment, moving_parent, placing_segment)) = placing_segments.get_single_mut()
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
            .find(|vertex| vertex.distance(point) < placing_segment.snap_offset)
            .unwrap_or(point);

        trace!(
            "updating `{:?}` to `{snapped_point:?}`",
            placing_segment.point_kind
        );
        segment.set_point(placing_segment.point_kind, snapped_point);
    }
}

#[derive(Component)]
pub(crate) struct PlacingSegment {
    pub(crate) point_kind: PointKind,
    pub(crate) snap_offset: f32,
}

impl InputContext for PlacingSegment {
    const PRIORITY: isize = 1;

    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        ctx.bind::<DeleteSegment>()
            .to((&settings.keyboard.delete, GamepadButtonType::North));
        ctx.bind::<CancelSegment>()
            .to((KeyCode::Escape, GamepadButtonType::East));
        ctx.bind::<ConfirmSegment>()
            .to((MouseButton::Left, GamepadButtonType::South));

        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub(crate) struct DeleteSegment;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub(crate) struct CancelSegment;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub(crate) struct ConfirmSegment;
