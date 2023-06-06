use bevy::{app::AppExit, prelude::*};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    theme::Theme,
    ui_state::UiState,
    widget::{
        button::{ButtonSize, TextButtonBundle},
        ui_root::UiRoot,
    },
};

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::setup_system.in_schedule(OnEnter(UiState::MainMenu)),
            Self::button_system.in_set(OnUpdate(UiState::MainMenu)),
        ));
    }
}

impl MainMenuPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        size: Size::all(Val::Percent(100.0)),
                        align_items: AlignItems::FlexStart,
                        justify_content: JustifyContent::Center,
                        padding: theme.global_padding,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                for button in MainMenuButton::iter() {
                    parent.spawn((
                        button,
                        TextButtonBundle::new(&theme, ButtonSize::Large, button.to_string()),
                    ));
                }
            });
    }

    fn button_system(
        mut exit_events: EventWriter<AppExit>,
        mut ui_state: ResMut<NextState<UiState>>,
        buttons: Query<(&Interaction, &MainMenuButton), Changed<Interaction>>,
    ) {
        for (interaction, button) in &buttons {
            if *interaction == Interaction::Clicked {
                match button {
                    MainMenuButton::Play => ui_state.set(UiState::WorldBrowser),
                    MainMenuButton::Settings => ui_state.set(UiState::Settings),
                    MainMenuButton::Exit => exit_events.send_default(),
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Display, EnumIter)]
enum MainMenuButton {
    Play,
    Settings,
    Exit,
}
