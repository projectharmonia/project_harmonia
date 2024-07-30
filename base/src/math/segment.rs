use std::ops::{Add, Sub};

use bevy::prelude::*;
use itertools::MinMaxResult;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Deserialize, Reflect, Serialize)]
pub(crate) struct Segment {
    pub(crate) start: Vec2,
    pub(crate) end: Vec2,
}

impl Segment {
    /// Creates a new segment by endpoints.
    pub(crate) fn new(start: Vec2, end: Vec2) -> Self {
        Self { start, end }
    }

    /// Creates a segment with the same start and end points.
    pub(crate) fn splat(point: Vec2) -> Self {
        Self {
            start: point,
            end: point,
        }
    }

    /// Returns `true` if a point belongs to a segment.
    pub(crate) fn contains(&self, point: Vec2) -> bool {
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
    pub(crate) fn closest_point(&self, point: Vec2) -> Vec2 {
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
    pub(crate) fn inverse(&self) -> Self {
        Self {
            start: self.end,
            end: self.start,
        }
    }

    /// Calculates displacement vector of the segment.
    pub(crate) fn displacement(&self) -> Vec2 {
        self.end - self.start
    }

    /// Returns the intersection point of lines constructed from segments.
    pub(crate) fn line_intersection(&self, other: Self) -> Option<Vec2> {
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
    pub(crate) fn intersects(&self, other: Self) -> bool {
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
    pub(crate) fn offset_points(
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
    pub(crate) fn points(&self) -> [Vec2; 2] {
        [self.start, self.end]
    }

    /// Calculates the slope (Δy/Δx).
    fn slope(&self) -> f32 {
        (self.end.y - self.start.y) / (self.end.x - self.start.x)
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
