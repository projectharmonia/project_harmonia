use bevy::prelude::*;
use bevy_mod_outline::{OutlineBundle, OutlineVolume};

use super::{cursor_hover::CursorHover, game_state::GameState};

pub(super) struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (Self::enable_system, Self::disable_system)
                .run_if(in_state(GameState::City).or_else(in_state(GameState::Family))),
        );
    }
}

impl HighlightingPlugin {
    fn enable_system(
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
        hovered: Query<Entity, Added<CursorHover>>,
    ) {
        if let Ok(hovered_entity) = hovered.get_single() {
            for child_entity in children.iter_descendants(hovered_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = true;
                }
            }
        }
    }

    fn disable_system(
        mut unhovered: RemovedComponents<CursorHover>,
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
    ) {
        for parent_entity in unhovered.read() {
            for child_entity in children.iter_descendants(parent_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = false;
                }
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
