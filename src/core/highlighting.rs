use bevy::prelude::*;
use bevy_mod_outline::{OutlineBundle, OutlineVolume};

use super::{cursor_hover::CursorHover, game_state::GameState};

pub(super) struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (Self::enable_system, Self::disable_system)
                .run_if(in_state(GameState::City).or_else(in_state(GameState::Family))),
        );
    }
}

impl HighlightingPlugin {
    fn enable_system(mut hovered: Query<&mut OutlineVolume, Added<CursorHover>>) {
        if let Ok(mut outline) = hovered.get_single_mut() {
            outline.visible = true;
        }
    }

    fn disable_system(
        mut unhovered: RemovedComponents<CursorHover>,
        mut hovered: Query<&mut OutlineVolume>,
    ) {
        for entity in unhovered.read() {
            if let Ok(mut outline) = hovered.get_mut(entity) {
                outline.visible = false;
            }
        }
    }
}

pub(super) trait OutlineHighlightingExt {
    fn highlighting() -> Self;
}

impl OutlineHighlightingExt for OutlineBundle {
    fn highlighting() -> Self {
        Self {
            outline: OutlineVolume {
                visible: false,
                colour: Color::rgba(1.0, 1.0, 1.0, 0.3),
                width: 2.0,
            },
            ..Default::default()
        }
    }
}
