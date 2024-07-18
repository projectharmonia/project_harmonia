use bevy::{color::palettes::css::LIMEGREEN, prelude::*};

use super::NavPath;
use crate::settings::Settings;

pub(super) struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Self::draw_lines.run_if(debug_paths_enabled()));
    }
}

impl PathDebugPlugin {
    fn draw_lines(mut gizmos: Gizmos, actors: Query<&NavPath>) {
        for nav_path in &actors {
            gizmos.linestrip(nav_path.0.iter().copied(), LIMEGREEN);
        }
    }
}

fn debug_paths_enabled() -> impl FnMut(Res<Settings>) -> bool {
    |settings| settings.developer.debug_paths
}
