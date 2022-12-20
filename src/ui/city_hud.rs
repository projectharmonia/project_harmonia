mod objects_tab;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use strum::IntoEnumIterator;

use crate::core::{
    asset_metadata::AssetMetadata,
    game_state::{CursorMode, GameState},
    object::selected_object::SelectedObject,
    preview::{PreviewRequest, Previews},
};
use objects_tab::ObjectsTab;

pub(super) struct CityHudPlugin;

impl Plugin for CityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::bottom_panel_system.run_in_state(GameState::City));
    }
}

impl CityHudPlugin {
    fn bottom_panel_system(
        mut commands: Commands,
        mut preview_events: EventWriter<PreviewRequest>,
        mut egui: ResMut<EguiContext>,
        previews: Res<Previews>,
        cursor_mode: Res<CurrentState<CursorMode>>,
        metadata: Res<Assets<AssetMetadata>>,
        selected_object: Option<Res<SelectedObject>>,
    ) {
        Window::new("City bottom panel")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let mut current_mode = cursor_mode.0;
                        for mode in CursorMode::iter() {
                            ui.selectable_value(
                                &mut current_mode,
                                mode,
                                RichText::new(mode.glyph()).size(22.0),
                            )
                            .on_hover_text(mode.to_string());
                        }
                        if current_mode != cursor_mode.0 {
                            commands.insert_resource(NextState(current_mode))
                        }
                    });
                    match cursor_mode.0 {
                        CursorMode::Objects => {
                            ObjectsTab::new(
                                &mut commands,
                                &metadata,
                                &previews,
                                &mut preview_events,
                                selected_object.map(|object| object.0),
                            )
                            .show(ui);
                        }
                        CursorMode::Lots => (),
                    }
                });
            });
    }
}
