use bevy::prelude::*;
use bevy_mod_outline::{OutlineBundle, OutlineVolume};

use crate::game_world::hover::Hovered;

pub(super) struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::enable).observe(Self::disable);
    }
}

impl HighlightingPlugin {
    fn enable(trigger: Trigger<OnAdd, Hovered>, mut hovered: Query<&mut OutlineVolume>) {
        if let Ok(mut outline) = hovered.get_mut(trigger.entity()) {
            debug!("highlighting enabled");
            outline.visible = true;
        }
    }

    fn disable(trigger: Trigger<OnRemove, Hovered>, mut hovered: Query<&mut OutlineVolume>) {
        if let Ok(mut outline) = hovered.get_mut(trigger.entity()) {
            debug!("highlighting disabled");
            outline.visible = false;
        }
    }
}

pub(crate) trait OutlineHighlightingExt {
    fn highlighting() -> Self;
}

impl OutlineHighlightingExt for OutlineBundle {
    fn highlighting() -> Self {
        Self {
            outline: OutlineVolume {
                visible: false,
                colour: Color::srgba(1.0, 1.0, 1.0, 0.3),
                width: 3.0,
            },
            ..Default::default()
        }
    }
}
