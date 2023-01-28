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
    city::ActiveCity,
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    object::placing_object::PlacingObject,
    preview::{PreviewRequest, Previews},
};

pub(super) struct BuildingHudPlugin;

impl Plugin for BuildingHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::bottom_panel_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        );
    }
}

impl BuildingHudPlugin {
    fn bottom_panel_system(
        mut current_category: Local<Option<ObjectCategory>>,
        mut commands: Commands,
        mut preview_events: EventWriter<PreviewRequest>,
        mut egui: ResMut<EguiContext>,
        previews: Res<Previews>,
        building_mode: Res<CurrentState<BuildingMode>>,
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
                            commands.insert_resource(NextState(current_mode))
                        }
                    });
                    if building_mode.0 == BuildingMode::Objects {
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
                                .and_then(|object| object.spawning_id()),
                            active_cities.single(),
                        )
                        .show(ui);
                    }
                });
            });
    }
}
