mod animation_state;
pub mod asset;
mod component_commands;
pub mod core;
pub mod game_paths;
pub mod game_world;
pub mod input_events;
mod math;
pub mod message;
mod navigation;
pub mod network;
pub mod settings;

use bevy::{app::PluginGroupBuilder, prelude::*};

use animation_state::AnimationStatePlugin;
use asset::AssetPlugin;
use core::CorePlugin;
use game_paths::GamePathsPlugin;
use game_world::GameWorldPlugin;
use math::MathPlugin;
use message::ErrorReportPlugin;
use navigation::NavigationPlugin;
use settings::SettingsPlugin;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(MathPlugin)
            .add(CorePlugin)
            .add(GameWorldPlugin)
            .add(AnimationStatePlugin)
            .add(NavigationPlugin)
            .add(ErrorReportPlugin)
            .add(GamePathsPlugin)
            .add(SettingsPlugin)
            .add(AssetPlugin) // Should run after registering components.
    }
}
