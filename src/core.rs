pub(super) mod asset_metadata;
pub(super) mod city;
pub(super) mod cli;
pub(super) mod control_action;
pub(super) mod developer;
pub(super) mod doll;
pub(super) mod error_message;
pub(super) mod family;
pub(super) mod family_editor;
pub(super) mod game_paths;
pub(super) mod game_state;
pub(super) mod game_world;
pub(super) mod ground;
pub(super) mod input_events;
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
use doll::DollPlugin;
use family::FamilyPlugin;
use family_editor::FamilyEditorPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use game_world::GameWorldPlugins;
use ground::GroundPlugin;
use network::NetworkPlugins;
use object::ObjectPlugins;
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
            .add(DollPlugin)
            .add(GroundPlugin)
            .add(ControlActionsPlugin)
            .add(DeveloperPlugin)
            .add(FamilyPlugin)
            .add(FamilyEditorPlugin)
            .add(GamePathsPlugin)
            .add(PreviewPlugin)
            .add(OrbitCameraPlugin)
            .add(SettingsPlugin);

        GameWorldPlugins.build(group);
        ObjectPlugins.build(group);
        NetworkPlugins.build(group);
    }
}
