use bevy::{
    prelude::*,
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::{InheritOutlineBundle, OutlineBundle, OutlineVolume};

use crate::{core::GameState, game_world::hover::Hovered};

pub(super) struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::enable)
            .observe(Self::disable)
            .add_systems(
                SpawnScene,
                Self::init_scene
                    .run_if(in_state(GameState::InGame))
                    .after(scene::scene_spawner_system),
            );
    }
}

impl HighlightingPlugin {
    /// Initializes scene children with [`InheritOutlineBundle`] to let toggle only top-level entity.
    fn init_scene(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        scenes: Query<Entity, With<OutlineVolume>>,
        chidlren: Query<&Children>,
    ) {
        for scene_entity in scenes.iter_many(ready_events.read().map(|event| event.parent)) {
            debug!("initializing outline for scene `{scene_entity}`");
            for child_entity in chidlren.iter_descendants(scene_entity) {
                commands
                    .entity(child_entity)
                    .insert(InheritOutlineBundle::default());
            }
        }
    }

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
