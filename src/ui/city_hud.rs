use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use strum::IntoEnumIterator;

use super::objects_view::ObjectsView;
use crate::core::{
    asset_metadata::{ObjectCategory, ObjectMetadata},
    city::{ActiveCity, CityMode},
    game_state::GameState,
    lot::LotTool,
    object::placing_object::PlacingObject,
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
        city_mode: Res<CurrentState<CityMode>>,
        lot_tool: Res<CurrentState<LotTool>>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        placing_objects: Query<&PlacingObject>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        Window::new("City bottom panel")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let mut current_mode = city_mode.0;
                        for mode in CityMode::iter() {
                            ui.selectable_value(
                                &mut current_mode,
                                mode,
                                RichText::new(mode.glyph()).size(22.0),
                            )
                            .on_hover_text(mode.to_string());
                        }
                        if current_mode != city_mode.0 {
                            commands.insert_resource(NextState(current_mode))
                        }
                    });
                    match city_mode.0 {
                        CityMode::Objects => {
                            ObjectsView::new(
                                &mut current_category,
                                ObjectCategory::CITY_CATEGORIES,
                                &mut commands,
                                &object_metadata,
                                &previews,
                                &mut preview_events,
                                placing_objects
                                    .get_single()
                                    .ok()
                                    .and_then(PlacingObject::spawning_id),
                                active_cities.single(),
                            )
                            .show(ui);
                        }
                        CityMode::Lots => {
                            ui.vertical(|ui| {
                                let mut current_tool = lot_tool.0;
                                for tool in LotTool::iter() {
                                    ui.selectable_value(&mut current_tool, tool, tool.glyph())
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
