pub(super) mod task_menu;

use bevy::prelude::*;
use strum::IntoEnumIterator;

use super::{
    theme::Theme,
    widget::{
        button::{ExclusiveButton, TextButtonBundle, Toggled},
        ui_root::UiRoot,
    },
};
use crate::core::{family::FamilyMode, game_state::GameState};
use task_menu::TaskMenuPlugin;

pub(super) struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(TaskMenuPlugin).add_systems((
            Self::setup_system
                .run_if(not(any_with_component::<UiRoot>()))
                .in_schedule(OnEnter(GameState::Family)),
            Self::setup_system
                .run_if(not(any_with_component::<UiRoot>()))
                .in_schedule(OnEnter(GameState::City)),
            Self::mode_button_system
                .run_if(in_state(GameState::Family).or_else(in_state(GameState::City))),
        ));
    }
}

impl HudPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                UiRoot,
            ))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            position: UiRect::right(Val::Px(0.0)),
                            padding: theme.padding.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        for (index, mode) in FamilyMode::iter().enumerate() {
                            parent.spawn((
                                mode,
                                ExclusiveButton,
                                Toggled(index == 0),
                                TextButtonBundle::square(&theme, mode.glyph()),
                            ));
                        }
                    });
            });
    }

    fn mode_button_system(
        mut family_mode: ResMut<NextState<FamilyMode>>,
        buttons: Query<(&Toggled, &FamilyMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 {
                family_mode.set(mode);
            }
        }
    }
}
