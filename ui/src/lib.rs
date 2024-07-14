mod camera_2d;
mod error_dialog;
mod hud;
mod menu;
mod preview;
mod ui_root;

use bevy::{app::PluginGroupBuilder, prelude::*};

use camera_2d::Camera2dPlugin;
use error_dialog::ErrorDialogPlugin;
use hud::HudPlugin;
use menu::MenuPlugin;
use preview::PreviewPlugin;
use ui_root::UiRootPlugin;

pub struct UiPlugins;

impl PluginGroup for UiPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(Camera2dPlugin)
            .add(MenuPlugin)
            .add(ErrorDialogPlugin)
            .add(HudPlugin)
            .add(UiRootPlugin)
            .add(PreviewPlugin)
    }
}
