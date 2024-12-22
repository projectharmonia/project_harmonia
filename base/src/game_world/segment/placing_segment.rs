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
        instances: Res<ContextInstances>,
        mut placing_segments: Query<(Entity, &mut Segment, &Parent, &PlacingSegment)>,
        segments: Query<(&Parent, &Segment), Without<PlacingSegment>>,
    ) {
        let Ok((entity, mut segment, moving_parent, placing)) = placing_segments.get_single_mut()
        else {
            return;
        };

        let Some(mut new_point) = camera_caster.intersect_ground().map(|pos| pos.xz()) else {
            return;
        };

        // Use an already existing point if it is within the `snap_offset` distance if one exists.
        // Otherwise try to use rounded point.
        new_point = segments
            .iter()
            .filter(|(parent, _)| *parent == moving_parent)
            .flat_map(|(_, segment)| segment.points())
            .find(|point| point.distance(new_point) < placing.snap_offset)
            .unwrap_or_else(|| round(&instances, entity, *segment, *placing, new_point));

        trace!("updating `{:?}` to `{new_point:?}`", placing.point_kind);
        segment.set_point(placing.point_kind, new_point);
    }
}

fn round(
    instances: &ContextInstances,
    entity: Entity,
    segment: Segment,
    placing: PlacingSegment,
    point: Vec2,
) -> Vec2 {
    let ctx = instances.get::<PlacingSegment>(entity).unwrap();
    let action = ctx.action::<FreeSegmentPlacement>().unwrap();
    if action.state() == ActionState::Fired {
        // Use raw result.
        return point;
    }

    let origin = segment.point(placing.point_kind.inverse());
    const SNAP_LEN: f32 = 0.1;

    let disp = point - origin;
    let len = disp.length();
    let remainder = len % SNAP_LEN;
    if remainder == 0.0 {
        return point;
    }

    let adjustment = if remainder < SNAP_LEN / 2.0 {
        -remainder
    } else {
        SNAP_LEN - remainder
    };

    let snapped_len = len + adjustment;
    origin + disp.normalize() * snapped_len
}

#[derive(Component, Clone, Copy)]
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
        ctx.bind::<FreeSegmentPlacement>().to((
            &settings.keyboard.free_placement,
            GamepadButtonType::LeftTrigger2,
        ));

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

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct FreeSegmentPlacement;
