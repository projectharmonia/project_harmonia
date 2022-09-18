pub(super) mod asset_metadata;
pub(super) mod city;
pub(super) mod cli;
pub(super) mod control_action;
pub(super) mod developer;
pub(super) mod error;
pub(super) mod family;
pub(super) mod game_paths;
pub(super) mod game_state;
pub(super) mod game_world;
pub(super) mod ground;
pub(super) mod network;
pub(super) mod object;
pub(super) mod orbit_camera;
pub(super) mod preview;
pub(super) mod settings;

use bevy::{app::PluginGroupBuilder, prelude::*};

use asset_metadata::AssetMetadataPlugin;
use city::CityPlugin;
use cli::CliPlugin;
use control_action::ControlActionsPlugin;
use developer::DeveloperPlugin;
use family::FamilyPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use game_world::GameWorldPlugins;
use ground::GroundPlugin;
use network::NetworkPlugins;
use object::ObjectPlugin;
use orbit_camera::OrbitCameraPlugin;
use preview::PreviewPlugin;
use settings::SettingsPlugin;

pub(super) struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group
            .add(AssetMetadataPlugin)
            .add(GameStatePlugin)
            .add(CityPlugin)
            .add(CliPlugin)
            .add(GroundPlugin)
            .add(ObjectPlugin)
            .add(ControlActionsPlugin)
            .add(DeveloperPlugin)
            .add(FamilyPlugin)
            .add(GamePathsPlugin)
            .add(PreviewPlugin)
            .add(OrbitCameraPlugin)
            .add(SettingsPlugin);

        GameWorldPlugins.build(group);
        NetworkPlugins.build(group);
    }
}

#[cfg(test)]
mod tests {
    use bevy::{
        asset::{Asset, AssetPlugin, LoadState},
        core::CorePlugin,
        pbr::PbrPlugin,
        render::{settings::WgpuSettings, RenderPlugin},
        window::WindowPlugin,
    };
    use bevy_inspector_egui::WorldInspectorParams;
    use bevy_rapier3d::prelude::*;
    use leafwing_input_manager::plugin::InputManagerPlugin;

    use super::{cli::Cli, control_action::ControlAction, *};

    #[test]
    fn update() {
        App::new()
            .init_resource::<Cli>()
            .init_resource::<WorldInspectorParams>()
            .init_resource::<DebugRenderContext>()
            .add_plugin(HeadlessRenderPlugin)
            .add_plugin(InputManagerPlugin::<ControlAction>::default())
            .add_plugins(CorePlugins)
            .update();
    }

    pub(super) fn wait_for_asset_loading<T: Asset>(app: &mut App, handle: &Handle<T>) {
        loop {
            app.update();
            let asset_server = app.world.resource::<AssetServer>();
            match asset_server.get_load_state(handle) {
                LoadState::Loading | LoadState::NotLoaded => continue,
                LoadState::Loaded => return,
                LoadState::Failed => panic!("asset loading failed"),
                LoadState::Unloaded => {
                    unreachable!("asset shouldn't be unloaded while holding handle")
                }
            }
        }
    }
    // Allows to run tests for systems containing rendering related things without GPU
    pub(super) struct HeadlessRenderPlugin;

    impl Plugin for HeadlessRenderPlugin {
        fn build(&self, app: &mut App) {
            app.insert_resource(WgpuSettings {
                backends: None,
                ..Default::default()
            })
            .add_plugin(CorePlugin)
            .add_plugin(WindowPlugin)
            .add_plugin(AssetPlugin)
            .add_plugin(RenderPlugin)
            .add_plugin(PbrPlugin);
        }
    }
}
