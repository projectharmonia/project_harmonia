pub(super) mod segment;

use bevy::prelude::*;

use segment::Segment;

pub(super) struct MathPlugin;

impl Plugin for MathPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Segment>();
    }
}
