use bevy::{color::palettes::css::BLUE_VIOLET, prelude::*};

use super::NavPath;
use crate::{game_world::WorldState, settings::Settings};

pub(super) struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::draw_lines
                .run_if(in_state(WorldState::City).or_else(in_state(WorldState::Family)))
                .run_if(|settings: Res<Settings>| settings.developer.debug_paths),
        );
    }
}

impl PathDebugPlugin {
    fn draw_lines(mut gizmos: Gizmos, actors: Query<&NavPath>) {
        for path in &actors {
            gizmos.linestrip(path.iter().copied(), BLUE_VIOLET);
        }
    }
}
