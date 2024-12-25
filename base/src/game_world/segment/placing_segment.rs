use std::f32::consts::FRAC_PI_4;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::{CameraCaster, PointKind, Segment, SegmentConnections};
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
        mut placing_segments: Query<(
            Entity,
            &mut Segment,
            &SegmentConnections,
            &Parent,
            &PlacingSegment,
        )>,
        segments: Query<(&Parent, &Segment), Without<PlacingSegment>>,
    ) {
        let Ok((entity, mut segment, connections, moving_parent, placing)) =
            placing_segments.get_single_mut()
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
            .unwrap_or_else(|| {
                round_placement(
                    &instances,
                    entity,
                    *segment,
                    connections,
                    *placing,
                    new_point,
                )
            });

        trace!("updating `{:?}` to `{new_point:?}`", placing.point_kind);
        segment.set_point(placing.point_kind, new_point);
    }
}

fn round_placement(
    instances: &ContextInstances,
    entity: Entity,
    segment: Segment,
    connections: &SegmentConnections,
    placing: PlacingSegment,
    point: Vec2,
) -> Vec2 {
    let ctx = instances.get::<PlacingSegment>(entity).unwrap();
    let free_placement = ctx.action::<FreeSegmentPlacement>().unwrap();
    if free_placement.state() == ActionState::Fired {
        // Use raw result.
        return point;
    }

    let origin_kind = placing.point_kind.inverse();
    let origin = segment.point(placing.point_kind.inverse());
    if origin == point {
        return point;
    }

    let ordinal_placement = ctx.action::<OrdinalSegmentPlacement>().unwrap();
    let snap_angle = if ordinal_placement.state() == ActionState::Fired {
        FRAC_PI_4
    } else {
        5.0_f32.to_radians()
    };

    let rounded_point = round_angle(connections, origin, point, origin_kind, snap_angle);
    round_len(origin, rounded_point)
}

fn round_len(origin: Vec2, point: Vec2) -> Vec2 {
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

    let new_point = point + disp.normalize() * adjustment;
    trace!(
        "rounding len to {} by changing {point} to {new_point}",
        len + adjustment
    );

    new_point
}

fn round_angle(
    connections: &SegmentConnections,
    origin: Vec2,
    point: Vec2,
    origin_kind: PointKind,
    snap_angle: f32,
) -> Vec2 {
    let disp = point - origin;
    let angle = connections
        .min_angle(origin_kind, disp)
        .unwrap_or_else(|| -disp.angle_between(Vec2::X));

    let remainder = angle.abs() % snap_angle;
    if remainder == 0.0 {
        return point;
    }

    let adjustment = if remainder < snap_angle / 2.0 {
        -remainder
    } else {
        snap_angle - remainder
    };

    let rotation = Mat2::from_angle(adjustment * angle.signum());
    let new_point = origin + rotation * disp;
    trace!(
        "rounding angle to {} by changing {point} to {new_point}",
        (angle + adjustment * angle.signum()).to_degrees()
    );

    new_point
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
        ctx.bind::<OrdinalSegmentPlacement>().to((
            &settings.keyboard.ordinal_placement,
            GamepadButtonType::RightTrigger2,
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

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct OrdinalSegmentPlacement;
