use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use strum::IntoEnumIterator;

use super::objects_view::ObjectsView;
use crate::core::{
    asset_metadata::{AssetMetadata, ObjectCategory},
    game_state::{CursorMode, GameState},
    lot::LotTool,
    object::selected_object::SelectedObject,
    preview::{PreviewRequest, Previews},
};

pub(super) struct CityHudPlugin;

impl Plugin for CityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::bottom_panel_system.run_in_state(GameState::City));
    }
}

impl CityHudPlugin {
    fn bottom_panel_system(
        mut current_category: Local<Option<ObjectCategory>>,
        mut commands: Commands,
        mut preview_events: EventWriter<PreviewRequest>,
        mut egui: ResMut<EguiContext>,
        previews: Res<Previews>,
        cursor_mode: Res<CurrentState<CursorMode>>,
        lot_tool: Res<CurrentState<LotTool>>,
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
                            ObjectsView::new(
                                &mut current_category,
                                ObjectCategory::CITY_CATEGORIES,
                                &mut commands,
                                &metadata,
                                &previews,
                                &mut preview_events,
                                selected_object.map(|object| object.0),
                            )
                            .show(ui);
                        }
                        CursorMode::Lots => {
                            ui.vertical(|ui| {
                                let mut current_tool = lot_tool.0;
                                for tool in LotTool::iter() {
                                    ui.selectable_value(
                                        &mut current_tool,
                                        tool,
                                        RichText::new(tool.glyph()).size(22.0),
                                    )
                                    .on_hover_text(tool.to_string());
                                }
                                if current_tool != lot_tool.0 {
                                    commands.insert_resource(NextState(current_tool))
                                }
                            });
                        }
                    }
                });
            });
    }
}
