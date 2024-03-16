pub(super) mod polygon;
pub(super) mod segment;

use bevy::prelude::*;

use polygon::Polygon;
use segment::Segment;

pub(super) struct MathPlugin;

impl Plugin for MathPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Polygon>().register_type::<Segment>();
    }
}
