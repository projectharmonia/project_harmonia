mod objects_tab;

use bevy::prelude::*;
use bevy_egui::{
    egui::{Align2, RichText, Window},
    EguiContext,
};
use iyes_loopless::prelude::*;
use strum::{Display, EnumIter, IntoEnumIterator};

use crate::core::game_state::GameState;

use self::objects_tab::ObjectsTab;

pub(super) struct CityHudPlugin;

impl Plugin for CityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::bottom_panel_system.run_in_state(GameState::City));
    }
}

impl CityHudPlugin {
    fn bottom_panel_system(mut current_tab: Local<CityTab>, mut egui: ResMut<EguiContext>) {
        Window::new("City bottom panel")
            .resizable(false)
            .title_bar(false)
            .anchor(Align2::LEFT_BOTTOM, (0.0, 0.0))
            .show(egui.ctx_mut(), |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        for tab in CityTab::iter() {
                            ui.selectable_value(
                                &mut *current_tab,
                                tab,
                                RichText::new(tab.icon()).size(22.0),
                            )
                            .on_hover_text(tab.to_string());
                        }
                    });
                    match *current_tab {
                        CityTab::Objects => ObjectsTab.show(ui),
                        CityTab::Dolls | CityTab::Terrain | CityTab::Lots => (),
                    }
                });
            });
    }
}

#[derive(Default, Display, EnumIter, PartialEq, Clone, Copy)]
enum CityTab {
    #[default]
    Objects,
    Dolls,
    Terrain,
    Lots,
}

impl CityTab {
    fn icon(self) -> &'static str {
        match self {
            CityTab::Objects => "🌳",
            CityTab::Dolls => "👪",
            CityTab::Terrain => "⛰",
            CityTab::Lots => "⛶",
        }
    }
}
