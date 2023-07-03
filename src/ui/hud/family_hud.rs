mod building_hud;
mod life_hud;

use bevy::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    core::{family::FamilyMode, game_state::GameState},
    ui::{
        theme::Theme,
        widget::{
            button::{ExclusiveButton, TextButtonBundle, Toggled},
            ui_root::UiRoot,
        },
    },
};
use building_hud::BuildingHudPlugin;
use life_hud::LifeHudPlugin;

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(BuildingHudPlugin)
            .add_plugin(LifeHudPlugin)
            .add_system(Self::setup_system.in_schedule(OnEnter(GameState::Family)))
            .add_system(Self::mode_button_system.in_set(OnUpdate(GameState::Family)));

        for state in FamilyMode::iter() {
            app.add_system(
                Self::cleanup_system
                    .run_if(in_state(GameState::Family))
                    .in_schedule(OnExit(state)),
            );
        }
    }
}

impl FamilyHudPlugin {
    fn setup_system(mut commands: Commands, theme: Res<Theme>) {
        commands
            .spawn((
                UiRoot,
                NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((FamilyHudRoot, NodeBundle::default()));

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
                        for mode in FamilyMode::iter() {
                            parent.spawn((
                                mode,
                                ExclusiveButton,
                                Toggled(mode == Default::default()),
                                TextButtonBundle::square(&theme, mode.glyph()),
                            ));
                        }
                    });
            });
    }

    fn mode_button_system(
        mut family_mode: ResMut<NextState<FamilyMode>>,
        buttons: Query<(Ref<Toggled>, &FamilyMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                family_mode.set(mode);
            }
        }
    }

    fn cleanup_system(mut commands: Commands, roots: Query<Entity, With<FamilyHudRoot>>) {
        commands.entity(roots.single()).despawn_descendants();
    }
}

#[derive(Component)]
struct FamilyHudRoot;
