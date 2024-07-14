pub mod action;
pub mod actor;
mod animation_state;
pub mod asset;
pub mod city;
mod component_commands;
pub mod cursor_hover;
pub mod developer;
pub mod family;
pub mod game_paths;
pub mod game_state;
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

use action::ActionPlugin;
use actor::ActorPlugin;
use animation_state::AnimationStatePlugin;
use asset::AssetPlugin;
use city::CityPlugin;
use cursor_hover::CursorHoverPlugin;
use developer::DeveloperPlugin;
use family::FamilyPlugin;
use game_paths::GamePathsPlugin;
use game_state::GameStatePlugin;
use game_world::GameWorldPlugin;
use highlighting::HighlightingPlugin;
use lot::LotPlugin;
use math::MathPlugin;
use message::ErrorReportPlugin;
use navigation::NavigationPlugin;
use network::NetworkPlugin;
use object::ObjectPlugin;
use player_camera::PlayerCameraPlugin;
use settings::SettingsPlugin;
use wall::WallPlugin;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(MathPlugin)
            .add(NetworkPlugin)
            .add(GameStatePlugin)
            .add(GameWorldPlugin)
            .add(CityPlugin)
            .add(CursorHoverPlugin)
            .add(HighlightingPlugin)
            .add(ActorPlugin)
            .add(AnimationStatePlugin)
            .add(LotPlugin)
            .add(NavigationPlugin)
            .add(ActionPlugin)
            .add(DeveloperPlugin)
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
