use bevy::{app::AppExit, prelude::*};
use bevy_egui::{
    egui::{Align2, Area, Button, RichText, TextStyle},
    EguiContext,
};
use iyes_loopless::prelude::*;

use super::{settings_menu::SettingsMenu, world_browser::WorldBrowser, UI_MARGIN};
use crate::core::game_state::GameState;

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::main_menu_system.run_in_state(GameState::Menu));
    }
}

impl MainMenuPlugin {
    fn main_menu_system(
        mut commands: Commands,
        mut exit_events: EventWriter<AppExit>,
        mut egui: ResMut<EguiContext>,
    ) {
        Area::new("Main Menu")
            .anchor(Align2::LEFT_CENTER, (UI_MARGIN, 0.0))
            .show(egui.ctx_mut(), |ui| {
                if ui
                    .add(Button::new(
                        RichText::new("Play").text_style(TextStyle::Heading),
                    ))
                    .clicked()
                {
                    commands.init_resource::<WorldBrowser>();
                }
                if ui
                    .add(Button::new(
                        RichText::new("Settings").text_style(TextStyle::Heading),
                    ))
                    .clicked()
                {
                    commands.init_resource::<SettingsMenu>();
                }
                if ui
                    .add(Button::new(
                        RichText::new("Exit").text_style(TextStyle::Heading),
                    ))
                    .clicked()
                {
                    exit_events.send_default();
                }
            });
    }
}
