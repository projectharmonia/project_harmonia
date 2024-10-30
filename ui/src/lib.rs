mod camera_2d;
mod error_dialog;
mod hud;
mod menu;
mod preview;
mod root;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use error_dialog::MessageBoxPlugin;
use hud::HudPlugin;
use menu::MenuPlugin;
use preview::PreviewPlugin;
use root::RootPlugin;

pub struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(Camera2dPlugin)
            .add(MenuPlugin)
            .add(MessageBoxPlugin)
            .add(HudPlugin)
            .add(PreviewPlugin)
            .add(RootPlugin)
    }
}
