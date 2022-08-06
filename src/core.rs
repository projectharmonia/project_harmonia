pub(super) mod city;
pub(super) mod developer;
pub(super) mod errors;
pub(super) mod family;
pub(super) mod game_paths;
pub(super) mod game_state;
pub(super) mod game_world;
pub(super) mod settings;

use bevy::{app::PluginGroupBuilder, prelude::*};

use city::CityPlugin;
use developer::DeveloperPlugin;
use family::FamilyPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use game_world::GameWorldPlugin;
use settings::SettingsPlugin;

pub(super) struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(CityPlugin)
            .add(DeveloperPlugin)
            .add(FamilyPlugin)
            .add(GamePathsPlugin)
            .add(GameStatePlugin)
            .add(GameWorldPlugin)
            .add(SettingsPlugin);
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
