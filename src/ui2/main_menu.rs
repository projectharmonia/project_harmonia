use bevy::prelude::*;

use crate::core::game_state::GameState;

use super::theme::Theme;

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((Self::setup_system.in_schedule(OnEnter(GameState::MainMenu)),));
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
                        ..Default::default()
                    },
                    ..Default::default()
                },
                MainMenu,
            ))
            .with_children(|parent| {
                parent
                    .spawn((theme.large_button(), MainMenuButton::Play))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "New Game",
                            theme.text.large.clone(),
                        ));
                    });
                parent
                    .spawn((theme.large_button(), MainMenuButton::Settings))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Settings",
                            theme.text.large.clone(),
                        ));
                    });
                parent
                    .spawn((theme.large_button(), MainMenuButton::Exit))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section("Exit", theme.text.large.clone()));
                    });
            });
    }
}

#[derive(Component)]
struct MainMenu;

#[derive(Component)]
enum MainMenuButton {
    Play,
    Settings,
    Exit,
}
