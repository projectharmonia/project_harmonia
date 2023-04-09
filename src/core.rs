pub(super) mod action;
pub(super) mod actor;
mod asset_handles;
pub(super) mod asset_metadata;
pub(super) mod city;
pub(super) mod cli;
mod collision_groups;
mod component_commands;
pub(super) mod condition;
pub(super) mod cursor_hover;
pub(super) mod developer;
pub(super) mod error;
pub(super) mod family;
pub(super) mod family_editor;
pub(super) mod game_paths;
pub(super) mod game_state;
pub(super) mod game_world;
mod ground;
pub(super) mod input_events;
pub(super) mod lot;
pub(super) mod network;
pub(super) mod object;
mod player_camera;
pub(super) mod preview;
pub(super) mod settings;
pub(super) mod task;
mod unique_asset;
pub(super) mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};

use action::ActionPlugin;
use actor::ActorPlugin;
use asset_metadata::AssetMetadataPlugin;
use city::CityPlugin;
use cli::CliPlugin;
use cursor_hover::CursorHoverPlugin;
use developer::DeveloperPlugin;
use family::FamilyPlugin;
use family_editor::FamilyEditorPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use game_world::GameWorldPlugin;
use ground::GroundPlugin;
use lot::LotPlugin;
use network::NetworkPlugin;
use object::ObjectPlugin;
use player_camera::PlayerCameraPlugin;
use preview::PreviewPlugin;
use settings::SettingsPlugin;
use task::TaskPlugin;
use unique_asset::UniqueAssetPlugin;
use wall::WallPlugin;

pub(super) struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(NetworkPlugin)
            .add(GameStatePlugin)
            .add(GameWorldPlugin)
            .add(CityPlugin)
            .add(CliPlugin)
            .add(CursorHoverPlugin)
            .add(ActorPlugin)
            .add(LotPlugin)
            .add(GroundPlugin)
            .add(ActionPlugin)
            .add(DeveloperPlugin)
            .add(FamilyPlugin)
            .add(FamilyEditorPlugin)
            .add(GamePathsPlugin)
            .add(PreviewPlugin)
            .add(PlayerCameraPlugin)
            .add(SettingsPlugin)
            .add(TaskPlugin)
            .add(ObjectPlugin)
            .add(UniqueAssetPlugin::<StandardMaterial>::default())
            .add(WallPlugin)
            .add(AssetMetadataPlugin) // Should run after registering components.
    }
}
