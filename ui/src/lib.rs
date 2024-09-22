mod camera_2d;
mod error_dialog;
mod fps_counter;
mod hud;
mod menu;
mod preview;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use error_dialog::MessageBoxPlugin;
use fps_counter::FpsCounterPlugin;
use hud::HudPlugin;
use menu::MenuPlugin;
use preview::PreviewPlugin;

pub struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(Camera2dPlugin)
            .add(MenuPlugin)
            .add(FpsCounterPlugin::default())
            .add(MessageBoxPlugin)
            .add(HudPlugin)
            .add(PreviewPlugin)
    }
}
