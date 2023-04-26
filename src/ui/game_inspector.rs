use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::{
    egui::{ScrollArea, Window},
    EguiContexts,
};
use bevy_inspector_egui::bevy_inspector;

use crate::core::settings::Settings;

pub(super) struct GameInspectorPlugin;

impl Plugin for GameInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::game_inspector_system
                .run_if(|settings: Res<Settings>| settings.developer.game_inspector),
        );
    }
}

impl GameInspectorPlugin {
    fn game_inspector_system(world: &mut World, state: &mut SystemState<EguiContexts>) {
        let egui_ctx = state.get_mut(world).ctx_mut().clone();
        Window::new("Game inspector")
            .default_size((320.0, 160.0))
            .show(&egui_ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    bevy_inspector::ui_for_world(world, ui);
                    ui.allocate_space(ui.available_size());
                });
            });
    }
}
