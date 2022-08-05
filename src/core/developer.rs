use bevy::prelude::*;
use bevy_inspector_egui::WorldInspectorParams;
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;

use super::settings::{Settings, SettingsApplied};

/// Propagates developer settings changes into resources.
pub(super) struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::toggle_inspector_system)
            .add_startup_system(Self::toggle_debug_collisions_system)
            .add_system(Self::toggle_inspector_system.run_on_event::<SettingsApplied>())
            .add_system(Self::toggle_debug_collisions_system.run_on_event::<SettingsApplied>());
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

#[cfg(test)]
mod tests {
    use bevy::ecs::event::Events;

    use super::*;

    #[test]
    fn settings_propagation() {
        let mut app = App::new();
        app.init_resource::<WorldInspectorParams>()
            .init_resource::<DebugRenderContext>()
            .init_resource::<Settings>()
            .add_event::<SettingsApplied>()
            .add_plugin(DeveloperPlugin);

        app.update();

        let world_inspector = app.world.resource::<WorldInspectorParams>().enabled;
        let debug_collisions = app.world.resource::<DebugRenderContext>().enabled;
        let mut settings = app.world.resource_mut::<Settings>();
        assert_eq!(
            settings.developer.world_inspector, world_inspector,
            "World inspector setting should be loaded at startup"
        );
        assert_eq!(
            settings.developer.debug_collisions, debug_collisions,
            "Debug collisions setting should be loaded at startup"
        );

        settings.developer.world_inspector = !settings.developer.world_inspector;
        settings.developer.debug_collisions = !settings.developer.debug_collisions;

        let mut apply_events = app.world.resource_mut::<Events<SettingsApplied>>();
        apply_events.send_default();

        app.update();

        let world_inspector = app.world.resource::<WorldInspectorParams>().enabled;
        let debug_collisions = app.world.resource::<DebugRenderContext>().enabled;
        let settings = app.world.resource::<Settings>();
        assert_eq!(
            settings.developer.world_inspector, world_inspector,
            "World inspector setting should be modified after applying settings"
        );
        assert_eq!(
            settings.developer.debug_collisions, debug_collisions,
            "Debug collisions setting should be modified after applying settings"
        );
    }
}
