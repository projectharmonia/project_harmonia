pub(super) mod button;
pub(super) mod checkbox;
pub(super) mod ui_root;

use bevy::prelude::*;

use button::ButtonPlugin;
use checkbox::CheckboxPlugin;
use ui_root::UiRootPlugin;

pub(super) struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ButtonPlugin)
            .add_plugin(CheckboxPlugin)
            .add_plugin(UiRootPlugin);
    }
}
