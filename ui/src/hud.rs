mod city_hud;
mod family_hud;
mod objects_node;
pub(super) mod task_menu;
mod tools_node;

use bevy::prelude::*;

use city_hud::CityHudPlugin;
use family_hud::FamilyHudPlugin;
use objects_node::ObjectsNodePlugin;
use task_menu::TaskMenuPlugin;
use tools_node::ToolsNodePlugin;

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            CityHudPlugin,
            ObjectsNodePlugin,
            FamilyHudPlugin,
            TaskMenuPlugin,
            ToolsNodePlugin,
        ));
    }
}
