use bevy::prelude::*;
use bevy_inspector_egui::WorldInspectorParams;
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;

use super::settings::{Settings, SettingsApply};

/// Propagates developer settings changes into resources.
pub(super) struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::toggle_inspector_system)
            .add_startup_system(Self::toggle_debug_collisions_system)
            .add_system(Self::toggle_inspector_system.run_on_event::<SettingsApply>())
            .add_system(Self::toggle_debug_collisions_system.run_on_event::<SettingsApply>());
    }
}

impl DeveloperPlugin {
    /// Enables or disables the world inspector when settings are applied.
    fn toggle_inspector_system(
        settings: Res<Settings>,
        mut world_inspector: ResMut<WorldInspectorParams>,
    ) {
        world_inspector.enabled = settings.developer.world_inspector;
    }

    /// Enables or disables collision debugging when settings are applied.
    fn toggle_debug_collisions_system(
        settings: Res<Settings>,
        mut debug_render_ctx: ResMut<DebugRenderContext>,
    ) {
        debug_render_ctx.enabled = settings.developer.debug_collisions;
    }
}
