pub(super) mod button;
pub(super) mod checkbox;

use bevy::prelude::*;

use button::ButtonPlugin;
use checkbox::CheckboxPlugin;

pub(super) struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ButtonPlugin).add_plugin(CheckboxPlugin);
    }
}
