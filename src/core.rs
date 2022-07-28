pub(super) mod developer;
pub(super) mod game_paths;
pub(super) mod game_state;
pub(super) mod settings;

use bevy::{app::PluginGroupBuilder, prelude::*};

use developer::DeveloperPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use settings::SettingsPlugin;

pub(super) struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(GamePathsPlugin)
            .add(SettingsPlugin)
            .add(GameStatePlugin)
            .add(DeveloperPlugin);
    }
}

#[cfg(test)]
mod tests {
    use bevy_inspector_egui::WorldInspectorParams;
    use bevy_rapier3d::prelude::*;

    use super::*;

    #[test]
    fn update() {
        App::new()
            .init_resource::<WorldInspectorParams>()
            .init_resource::<DebugRenderContext>()
            .add_plugins(CorePlugins)
            .update();
    }
}
