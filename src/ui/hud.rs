mod family_hud;
pub(super) mod task_menu;

use bevy::prelude::*;

use family_hud::FamilyHudPlugin;
use task_menu::TaskMenuPlugin;

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(FamilyHudPlugin).add_plugin(TaskMenuPlugin);
    }
}
