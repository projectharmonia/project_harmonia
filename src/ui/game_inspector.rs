use bevy::{ecs::system::SystemState, prelude::*};
use bevy_egui::{
    egui::{ScrollArea, Window},
    EguiContext,
};
use bevy_inspector_egui::bevy_inspector;
use iyes_loopless::prelude::*;

use crate::core::developer::GameInspector;

pub(super) struct GameInspectorPlugin;

impl Plugin for GameInspectorPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::game_inspector_system.run_if(game_inspector_enabled));
    }
}

impl GameInspectorPlugin {
    fn game_inspector_system(world: &mut World, state: &mut SystemState<ResMut<EguiContext>>) {
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

fn game_inspector_enabled(game_inspector: Res<GameInspector>) -> bool {
    game_inspector.enabled
}
