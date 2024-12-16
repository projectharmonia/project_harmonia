pub(crate) mod dynamic_mesh;

use std::{
    f32::consts::PI,
    mem,
    ops::{Add, Sub},
};

use bevy::{
    ecs::{
        component::{ComponentHooks, ComponentId, StorageType},
        world::DeferredWorld,
    },
    prelude::*,
};
use bevy_replicon::prelude::*;
use itertools::{Itertools, MinMaxResult};
use serde::{Deserialize, Serialize};

use crate::core::GameState;

pub(super) struct SplinePlugin;

impl Plugin for SplinePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Segment>()
            .replicate::<Segment>()
            .observe(Self::cleanup_connections)
            .add_systems(
                PostUpdate,
                Self::update_connections.run_if(in_state(GameState::InGame)),
            );
    }
}

impl SplinePlugin {
    /// Updates [`SplineConnections`] between segments.
    pub(super) fn update_connections(
        mut segments: Query<(Entity, &Visibility, &Segment, &mut SplineConnections)>,
        children: Query<&Children>,
        changed_segments: Query<
            (Entity, &Parent, &Visibility, &Segment),
            (
                Or<(Changed<Segment>, Changed<Visibility>)>,
                With<SplineConnections>,
            ),
        >,
    ) {
        for (segment_entity, parent, visibility, &segment) in &changed_segments {
            // Take changed connections to avoid mutability issues.
            let (.., mut connections) = segments
                .get_mut(segment_entity)
                .expect("query is a subset of the changed query");
            let mut taken_connections = mem::take(&mut *connections);

            // Cleanup old connections.
            for connection in taken_connections.0.drain(..) {
                let (.., mut other_connections) = segments
                    .get_mut(connection.entity)
                    .expect("connected segment should also have connections");
                if let Some(index) = other_connections.position(segment_entity) {
                    other_connections.0.remove(index);
                }
            }

            // If segment have zero length or hidden, exclude it from connections.
            if segment.start != segment.end && visibility != Visibility::Hidden {
                // Scan all segments from this lot for possible connections.
                let mut iter = segments.iter_many_mut(children.get(**parent).unwrap());
                while let Some((other_entity, visibility, &other_segment, mut other_connections)) =
                    iter.fetch_next()
                {
                    if visibility == Visibility::Hidden || segment_entity == other_entity {
                        // Don't connect to hidden segments or self.
                        continue;
                    }

                    let kind = if segment.start == other_segment.start {
                        (PointKind::Start, PointKind::Start)
                    } else if segment.start == other_segment.end {
                        (PointKind::Start, PointKind::End)
                    } else if segment.end == other_segment.end {
                        (PointKind::End, PointKind::End)
                    } else if segment.end == other_segment.start {
                        (PointKind::End, PointKind::Start)
                    } else {
                        continue;
                    };

                    trace!(
                        "connecting segments `{segment_entity}` and `{other_entity}` as `{kind:?}`"
                    );
                    taken_connections.0.push(SplineConnection {
                        entity: other_entity,
                        segment: other_segment,
                        kind,
                    });
                    other_connections.0.push(SplineConnection {
                        entity: segment_entity,
                        segment,
                        kind: (kind.1, kind.0),
                    });
                }
            }

            // Reinsert updated connections back.
            let (.., mut connections) = segments.get_mut(segment_entity).unwrap();
            *connections = taken_connections;
        }
    }

    fn cleanup_connections(
        trigger: Trigger<OnRemove, Segment>,
        mut entities_buffer: Local<Vec<Entity>>,
        mut segments: Query<&mut SplineConnections>,
    ) {
        let connections = segments.get(trigger.entity()).unwrap();
        entities_buffer.extend(connections.iter().map(|connection| connection.entity));

        debug!("removing connections for segment `{}`", trigger.entity());
        for entity in entities_buffer.drain(..) {
            if let Ok(mut connections) = segments.get_mut(entity) {
                let index = connections
                    .position(trigger.entity())
                    .expect("segment connection should be done both ways");

                connections.0.remove(index);
            }
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct Segment {
    pub(super) start: Vec2,
    pub(super) end: Vec2,
}

impl Segment {
    /// Creates a new segment by endpoints.
    pub(super) fn new(start: Vec2, end: Vec2) -> Self {
        Self { start, end }
    }

    /// Creates a segment with the same start and end points.
    pub(super) fn splat(point: Vec2) -> Self {
        Self {
            start: point,
            end: point,
        }
    }

    /// Returns `true` if a point belongs to a segment.
    pub(super) fn contains(&self, point: Vec2) -> bool {
        let disp = self.displacement();
        let point_disp = point - self.start;
        if disp.perp_dot(point_disp).abs() > 0.1 {
            return false;
        }

        let dot = disp.dot(point_disp);
        if dot < 0.0 {
            return false;
        }

        dot <= disp.length_squared()
    }

    /// Returns the closest point on the segment to a point.
    pub(super) fn closest_point(&self, point: Vec2) -> Vec2 {
        let disp = self.displacement();
        let dir = disp.normalize();
        let point_dir = point - self.start;
        let dot = dir.dot(point_dir);

        if dot <= 0.0 {
            self.start
        } else if dot >= disp.length() {
            self.end
        } else {
            self.start + dir * dot
        }
    }

    /// Swaps end and start.
    pub(super) fn inverse(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
        }
    }

    /// Calculates displacement vector of the segment.
    pub(super) fn displacement(&self) -> Vec2 {
        self.end - self.start
    }

    /// Returns the intersection point of lines constructed from segments.
    pub(super) fn line_intersection(&self, other: Self) -> Option<Vec2> {
        let slope1 = self.slope();
        let slope2 = other.slope();

        if slope1 == slope2 {
            return None; // Parallel lines, no intersection point
        }

        let start1 = self.start;
        let start2 = other.start;

        let x =
            ((slope1 * start1.x) - (slope2 * start2.x) + start2.y - start1.y) / (slope1 - slope2);
        let y = slope1 * (x - start1.x) + start1.y;

        Some(Vec2 { x, y })
    }

    /// Returns `true` if two segments intersect.
    pub(super) fn intersects(&self, other: Self) -> bool {
        let Some(intersection) = self.line_intersection(other) else {
            return false;
        };

        let distance1 = self.start.distance(intersection) + intersection.distance(self.end);
        let distance2 = other.start.distance(intersection) + intersection.distance(other.end);

        const TOLERANCE: f32 = 0.01;
        distance1 - self.len() < TOLERANCE && distance2 - other.len() < TOLERANCE
    }

    /// Calculates the left and right points for the `start` point of the segment based on `half_width`,
    /// considering intersections with other segments.
    ///
    /// `width_disp` is the width displacement vector of the segment.
    /// `half_width` is the half-width of the points for other segments.
    pub(super) fn offset_points(
        self,
        width_disp: Vec2,
        half_width: f32,
        connections: MinMaxResult<Segment>,
    ) -> (Vec2, Vec2) {
        match connections {
            MinMaxResult::NoElements => (self.start + width_disp, self.start - width_disp),
            MinMaxResult::OneElement(other_segment) => {
                let other_width = other_segment.displacement().perp().normalize() * half_width;
                let left = (self + width_disp)
                    .line_intersection(other_segment - other_width)
                    .unwrap_or_else(|| self.start + width_disp);
                let right = (self - width_disp)
                    .line_intersection(other_segment.inverse() + other_width)
                    .unwrap_or_else(|| self.start + width_disp);

                (left, right)
            }
            MinMaxResult::MinMax(min_segment, max_segment) => {
                let max_width = max_segment.displacement().perp().normalize() * half_width;
                let left = (self + width_disp)
                    .line_intersection(max_segment - max_width)
                    .unwrap_or_else(|| self.start + width_disp);
                let min_width = min_segment.displacement().perp().normalize() * half_width;
                let right = (self - width_disp)
                    .line_intersection(min_segment.inverse() + min_width)
                    .unwrap_or_else(|| self.start + width_disp);

                (left, right)
            }
        }
    }

    /// Returns distance from start to end.
    fn len(&self) -> f32 {
        self.start.distance(self.end)
    }

    // Returns start and end points.
    pub(super) fn points(&self) -> [Vec2; 2] {
        [self.start, self.end]
    }

    /// Calculates the slope (Δy/Δx).
    fn slope(&self) -> f32 {
        (self.end.y - self.start.y) / (self.end.x - self.start.x)
    }

    fn on_insert(mut world: DeferredWorld, entity: Entity, _component_id: ComponentId) {
        world
            .commands()
            .entity(entity)
            .insert(SplineConnections::default());
    }
}

impl Component for Segment {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(Self::on_insert);
    }
}

impl Add<Vec2> for Segment {
    type Output = Self;

    fn add(self, value: Vec2) -> Self {
        Segment {
            start: self.start + value,
            end: self.end + value,
        }
    }
}

impl Sub<Vec2> for Segment {
    type Output = Self;

    fn sub(self, value: Vec2) -> Self {
        Segment {
            start: self.start - value,
            end: self.end - value,
        }
    }
}

/// Dynamically updated component with precalculated connected entities for each segment point.
#[derive(Component, Default, Deref)]
pub(crate) struct SplineConnections(Vec<SplineConnection>);

impl SplineConnections {
    /// Returns the segments with the maximum and minimum angle relative
    /// to the displacement vector.
    pub(super) fn minmax_angles(&self, disp: Vec2, point_kind: PointKind) -> MinMaxResult<Segment> {
        self.0
            .iter()
            .filter(|connection| connection.kind.0 == point_kind)
            .map(|connection| {
                // Rotate points based on connection type.
                match connection.kind {
                    (PointKind::Start, PointKind::End) => connection.segment.inverse(),
                    (PointKind::End, PointKind::Start) => connection.segment,
                    (PointKind::Start, PointKind::Start) => connection.segment,
                    (PointKind::End, PointKind::End) => connection.segment.inverse(),
                }
            })
            .minmax_by_key(|segment| {
                let angle = segment.displacement().angle_between(disp);
                if angle < 0.0 {
                    angle + 2.0 * PI
                } else {
                    angle
                }
            })
    }

    fn position(&self, segment_entity: Entity) -> Option<usize> {
        self.iter()
            .position(|&SplineConnection { entity, .. }| entity == segment_entity)
    }
}

pub(crate) struct SplineConnection {
    entity: Entity,
    segment: Segment,
    kind: (PointKind, PointKind),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum PointKind {
    Start,
    End,
}
