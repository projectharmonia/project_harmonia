pub(super) mod segment;
pub(super) mod triangulator;

use bevy::prelude::*;

use segment::Segment;

pub(super) struct MathPlugin;

impl Plugin for MathPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Segment>();
    }
}
