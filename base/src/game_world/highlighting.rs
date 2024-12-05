use bevy::{
    prelude::*,
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::{InheritOutlineBundle, OutlineBundle, OutlineVolume};

use super::picking::{Hovered, Picked, Unhovered};
use crate::core::GameState;

pub(super) struct HighlightingPlugin;

impl Plugin for HighlightingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LastHighlighted>()
            .observe(Self::enable)
            .observe(Self::disable)
            .observe(Self::pick)
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
        children: Query<&Children>,
    ) {
        for scene_entity in scenes.iter_many(ready_events.read().map(|event| event.parent)) {
            debug!("initializing outline for scene `{scene_entity}`");
            for child_entity in children.iter_descendants(scene_entity) {
                commands
                    .entity(child_entity)
                    .insert(InheritOutlineBundle::default());
            }
        }
    }

    fn enable(
        trigger: Trigger<Hovered>,
        mut last_hovered: ResMut<LastHighlighted>,
        mut volumes: Query<&mut OutlineVolume>,
    ) {
        if let Ok(mut outline) = volumes.get_mut(trigger.entity()) {
            debug!("enabling highlighting for `{}`", trigger.entity());
            outline.visible = true;
            **last_hovered = Some(trigger.entity());
        }
    }

    fn disable(
        trigger: Trigger<Unhovered>,
        mut volumes: Query<&mut OutlineVolume>,
        mut last_hovered: ResMut<LastHighlighted>,
    ) {
        **last_hovered = None;
        if let Ok(mut outline) = volumes.get_mut(trigger.entity()) {
            debug!("disabling highlighting for `{}`", trigger.entity());
            outline.visible = false;
        }
    }

    fn pick(
        _trigger: Trigger<OnRemove, Picked>,
        mut last_hovered: ResMut<LastHighlighted>,
        mut volumes: Query<&mut OutlineVolume>,
    ) {
        if let Some(entity) = **last_hovered {
            debug!("clearing highlighting for `{entity}`");
            let mut outline = volumes
                .get_mut(entity)
                .expect("all hovered entities have outline");
            outline.visible = true;
            **last_hovered = Some(entity);
        }
    }
}

/// Stores last highlighted entity to cleanup when picking is disabled.
#[derive(Resource, Default, Deref, DerefMut)]
struct LastHighlighted(Option<Entity>);

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
