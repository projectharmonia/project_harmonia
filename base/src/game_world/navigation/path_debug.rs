use bevy::{color::palettes::css::BLUE_VIOLET, prelude::*};

use super::NavPath;
use crate::{common_conditions::in_any_state, game_world::WorldState, settings::Settings};

pub(super) struct PathDebugPlugin;

impl Plugin for PathDebugPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::draw_lines
                .run_if(in_any_state([WorldState::City, WorldState::Family]))
                .run_if(|settings: Res<Settings>| settings.developer.debug_paths),
        );
    }
}

impl PathDebugPlugin {
    fn draw_lines(
        mut gizmos: Gizmos,
        actors: Query<(&NavPath, &Parent)>,
        cities: Query<&GlobalTransform>,
    ) {
        for (path, parent) in &actors {
            let transform = cities.get(**parent).unwrap();
            gizmos.linestrip(
                path.iter().map(|&point| transform.transform_point(point)),
                BLUE_VIOLET,
            );
        }
    }
}
