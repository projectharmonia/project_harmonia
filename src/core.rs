pub(super) mod action;
pub(super) mod actor;
mod animation;
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
pub(super) mod game_paths;
pub(super) mod game_state;
pub(super) mod game_world;
mod ground;
pub(super) mod input_events;
pub(super) mod lot;
mod navigation;
pub(super) mod network;
pub(super) mod object;
mod player_camera;
pub(super) mod ready_scene;
mod reflect_bundle;
pub(super) mod settings;
pub(super) mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};

use action::ActionPlugin;
use actor::ActorPlugin;
use animation::AnimationPlugin;
use asset_metadata::AssetMetadataPlugin;
use city::CityPlugin;
use cli::CliPlugin;
use cursor_hover::CursorHoverPlugin;
use developer::DeveloperPlugin;
use error::ErrorPlugin;
use family::FamilyPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use game_world::GameWorldPlugin;
use ground::GroundPlugin;
use lot::LotPlugin;
use navigation::NavigationPlugin;
use network::NetworkPlugin;
use object::ObjectPlugin;
use player_camera::PlayerCameraPlugin;
use ready_scene::ReadyScenePlugin;
use settings::SettingsPlugin;
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
            .add(AnimationPlugin)
            .add(LotPlugin)
            .add(NavigationPlugin)
            .add(GroundPlugin)
            .add(ActionPlugin)
            .add(DeveloperPlugin)
            .add(ErrorPlugin)
            .add(FamilyPlugin)
            .add(GamePathsPlugin)
            .add(PlayerCameraPlugin)
            .add(ReadyScenePlugin)
            .add(SettingsPlugin)
            .add(ObjectPlugin)
            .add(WallPlugin)
            .add(AssetMetadataPlugin) // Should run after registering components.
    }
}
