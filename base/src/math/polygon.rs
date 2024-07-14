use bevy::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Deref, DerefMut, Deserialize, Reflect, Serialize)]
pub(crate) struct Polygon(pub(crate) Vec<Vec2>);

impl Polygon {
    /// A port of W. Randolph Franklin's [PNPOLY](https://wrf.ecse.rpi.edu//Research/Short_Notes/pnpoly.html) algorithm.
    #[must_use]
    pub(crate) fn contains_point(&self, point: Vec2) -> bool {
        let mut inside = false;
        for (a, b) in self.iter().tuple_windows() {
            if ((a.y > point.y) != (b.y > point.y))
                && (point.x < (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x)
            {
                inside = !inside;
            }
        }

        inside
    }
}

impl From<Vec<Vec2>> for Polygon {
    fn from(value: Vec<Vec2>) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_point() {
        let polygon = Polygon(vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 1.0),
        ]);
        assert!(polygon.contains_point(Vec2::new(1.2, 1.9)));
    }

    #[test]
    fn not_contains_point() {
        let polygon = Polygon(vec![
            Vec2::new(1.0, 1.0),
            Vec2::new(1.0, 2.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(2.0, 1.0),
        ]);
        assert!(!polygon.contains_point(Vec2::new(3.2, 4.9)));
    }
}
