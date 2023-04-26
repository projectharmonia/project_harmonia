use bevy::{pbr::wireframe::WireframeConfig, prelude::*};
use bevy_rapier3d::prelude::*;

use super::settings::{Settings, SettingsApplySet};

/// Propagates developer settings changes into resources.
pub(super) struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_systems((
            Self::debug_collisions_toggle_system,
            Self::wireframe_toggle_system,
        ))
        .add_systems(
            (
                Self::debug_collisions_toggle_system,
                Self::wireframe_toggle_system,
            )
                .in_set(SettingsApplySet),
        );
    }
}

impl DeveloperPlugin {
    fn debug_collisions_toggle_system(
        settings: Res<Settings>,
        mut debug_render_ctx: ResMut<DebugRenderContext>,
    ) {
        debug_render_ctx.enabled = settings.developer.debug_collisions;
    }

    fn wireframe_toggle_system(
        settings: Res<Settings>,
        mut wireframe_config: ResMut<WireframeConfig>,
    ) {
        wireframe_config.global = settings.developer.wireframe;
    }
}
