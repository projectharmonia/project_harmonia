pub mod actor;
mod animation_state;
pub mod asset;
pub mod city;
mod component_commands;
pub mod core;
pub mod cursor_hover;
pub mod family;
pub mod game_paths;
pub mod game_world;
mod highlighting;
pub mod input_events;
pub mod lot;
mod math;
pub mod message;
mod navigation;
pub mod network;
pub mod object;
mod player_camera;
pub mod settings;
pub mod wall;

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_xpbd_3d::prelude::*;

use actor::ActorPlugin;
use animation_state::AnimationStatePlugin;
use asset::AssetPlugin;
use city::CityPlugin;
use core::CorePlugin;
use cursor_hover::CursorHoverPlugin;
use family::FamilyPlugin;
use game_paths::GamePathsPlugin;
use game_world::GameWorldPlugin;
use highlighting::HighlightingPlugin;
use lot::LotPlugin;
use math::MathPlugin;
use message::ErrorReportPlugin;
use navigation::NavigationPlugin;
use object::ObjectPlugin;
use player_camera::PlayerCameraPlugin;
use settings::SettingsPlugin;
use wall::WallPlugin;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(MathPlugin)
            .add(CorePlugin)
            .add(GameWorldPlugin)
            .add(CityPlugin)
            .add(CursorHoverPlugin)
            .add(HighlightingPlugin)
            .add(ActorPlugin)
            .add(AnimationStatePlugin)
            .add(LotPlugin)
            .add(NavigationPlugin)
            .add(ErrorReportPlugin)
            .add(FamilyPlugin)
            .add(GamePathsPlugin)
            .add(PlayerCameraPlugin)
            .add(SettingsPlugin)
            .add(ObjectPlugin)
            .add(WallPlugin)
            .add(AssetPlugin) // Should run after registering components.
    }
}

#[derive(PhysicsLayer)]
enum Layer {
    Ground,
    Object,
    Wall,
}
