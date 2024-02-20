use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub(super) struct Line {
    a: f32,
    b: f32,
    c: f32,
}

impl Line {
    #[must_use]
    pub(super) fn new(p1: Vec2, p2: Vec2) -> Self {
        let a = p2.y - p1.y;
        let b = p1.x - p2.x;
        let c = a * p1.x + b * p1.y;
        Self { a, b, c }
    }

    #[must_use]
    pub(super) fn with_offset(p1: Vec2, p2: Vec2, offset: Vec2) -> Self {
        Self::new(p1 + offset, p2 + offset)
    }

    #[must_use]
    pub(super) fn intersection(self, rhs: Self) -> Option<Vec2> {
        let det = self.a * rhs.b - rhs.a * self.b;
        if det == 0.0 {
            None
        } else {
            Some(Vec2 {
                x: (rhs.b * self.c - self.b * rhs.c) / det,
                y: (self.a * rhs.c - rhs.a * self.c) / det,
            })
        }
    }
}
