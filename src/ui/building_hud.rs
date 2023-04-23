use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContexts,
};
use strum::IntoEnumIterator;

use super::objects_view::ObjectsView;
use crate::core::{
    asset_metadata::{ObjectCategory, ObjectMetadata},
    city::ActiveCity,
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    object::placing_object::PlacingObject,
    preview::Previews,
};

pub(super) struct BuildingHudPlugin;

impl Plugin for BuildingHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::bottom_panel_system
                .in_set(OnUpdate(GameState::Family))
                .in_set(OnUpdate(FamilyMode::Building)),
        );
    }
}

impl BuildingHudPlugin {
    fn bottom_panel_system(
        mut current_category: Local<Option<ObjectCategory>>,
        mut commands: Commands,
        mut egui: EguiContexts,
        mut next_building_mode: ResMut<NextState<BuildingMode>>,
        mut previews: ResMut<Previews>,
        building_mode: Res<State<BuildingMode>>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        placing_objects: Query<&PlacingObject>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        Window::new("Building bottom panel")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        let mut current_mode = building_mode.0;
                        for mode in BuildingMode::iter() {
                            ui.selectable_value(
                                &mut current_mode,
                                mode,
                                RichText::new(mode.glyph()).size(22.0),
                            )
                            .on_hover_text(mode.to_string());
                        }
                        if current_mode != building_mode.0 {
                            next_building_mode.set(current_mode);
                        }
                    });
                    if building_mode.0 == BuildingMode::Objects {
                        ObjectsView::new(
                            &mut current_category,
                            ObjectCategory::FAMILY_CATEGORIES,
                            &mut commands,
                            &object_metadata,
                            &mut previews,
                            placing_objects
                                .get_single()
                                .ok()
                                .and_then(PlacingObject::spawning_id),
                            active_cities.single(),
                        )
                        .show(ui);
                    }
                });
            });
    }
}
