use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;

use super::objects_view::ObjectsView;
use crate::core::{
    asset_metadata::{AssetMetadata, ObjectCategory},
    family::FamilyMode,
    game_state::GameState,
    object::selected_object::SelectedObject,
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
        metadata: Res<Assets<AssetMetadata>>,
        selected_object: Option<Res<SelectedObject>>,
    ) {
        Window::new("Building bottom panel")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
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
                });
            });
    }
}
