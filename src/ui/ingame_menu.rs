use bevy::{app::AppExit, prelude::*};
use leafwing_input_manager::common_conditions::action_just_pressed;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    hud::task_menu::TaskMenu,
    settings_menu::SettingsMenuOpen,
    theme::Theme,
    widget::{button::TextButtonBundle, click::Click, ui_root::UiRoot, DialogBundle, LabelBundle},
};
use crate::core::{
    action::Action,
    game_state::GameState,
    game_world::{GameSave, GameWorld},
    lot::{creating_lot::CreatingLot, moving_lot::MovingLot},
    object::placing_object::PlacingObject,
    wall::creating_wall::CreatingWall,
};

pub(super) struct InGameMenuPlugin;

impl Plugin for InGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::open
                    .run_if(action_just_pressed(Action::Cancel))
                    .run_if(not(any_with_component::<IngameMenu>))
                    .run_if(not(any_with_component::<TaskMenu>))
                    .run_if(not(any_with_component::<PlacingObject>))
                    .run_if(not(any_with_component::<CreatingLot>))
                    .run_if(not(any_with_component::<MovingLot>))
                    .run_if(not(any_with_component::<CreatingLot>))
                    .run_if(not(any_with_component::<CreatingWall>))
                    .run_if(in_state(GameState::Family).or_else(in_state(GameState::City))),
                (
                    Self::handle_menu_clicks,
                    Self::handle_exit_dialog_clicks,
                    Self::close
                        .run_if(not(any_with_component::<ExitDialog>))
                        .run_if(action_just_pressed(Action::Cancel)),
                )
                    .run_if(any_with_component::<IngameMenu>),
            ),
        );
    }
}

impl InGameMenuPlugin {
    fn open(mut commands: Commands, theme: Res<Theme>, roots: Query<Entity, With<UiRoot>>) {
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((IngameMenu, DialogBundle::new(&theme)))
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                padding: theme.padding.normal,
                                row_gap: theme.gap.normal,
                                ..Default::default()
                            },
                            background_color: theme.panel_color.into(),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn(LabelBundle::normal(&theme, "Main menu"));

                            for button in IngameMenuButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(&theme, button.to_string()),
                                ));
                            }
                        });
                });
        });
    }

    fn handle_menu_clicks(
        mut commands: Commands,
        mut save_events: EventWriter<GameSave>,
        mut settings_events: EventWriter<SettingsMenuOpen>,
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<&IngameMenuButton>,
        roots: Query<Entity, With<UiRoot>>,
        ingame_menus: Query<Entity, With<IngameMenu>>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                IngameMenuButton::Resume => {
                    commands.entity(ingame_menus.single()).despawn_recursive()
                }
                IngameMenuButton::Save => {
                    save_events.send_default();
                    commands.entity(ingame_menus.single()).despawn_recursive();
                }
                IngameMenuButton::Settings => {
                    settings_events.send_default();
                }
                IngameMenuButton::World => game_state.set(GameState::World),
                IngameMenuButton::MainMenu => {
                    setup_exit_dialog(&mut commands, roots.single(), &theme, ExitDialog::MainMenu)
                }
                IngameMenuButton::ExitGame => {
                    setup_exit_dialog(&mut commands, roots.single(), &theme, ExitDialog::Game)
                }
            }
        }
    }

    fn handle_exit_dialog_clicks(
        mut commands: Commands,
        mut save_events: EventWriter<GameSave>,
        mut exit_events: EventWriter<AppExit>,
        mut click_events: EventReader<Click>,
        mut game_state: ResMut<NextState<GameState>>,
        buttons: Query<&ExitDialogButton>,
        exit_dialogs: Query<(Entity, &ExitDialog)>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            let (dialog_entity, exit_dialog) = exit_dialogs.single();
            match button {
                ExitDialogButton::SaveAndExit => {
                    save_events.send_default();
                    match exit_dialog {
                        ExitDialog::MainMenu => {
                            commands.remove_resource::<GameWorld>();
                            game_state.set(GameState::MainMenu);
                        }
                        ExitDialog::Game => {
                            exit_events.send_default();
                        }
                    }
                }
                ExitDialogButton::Exit => match exit_dialog {
                    ExitDialog::MainMenu => {
                        commands.remove_resource::<GameWorld>();
                        game_state.set(GameState::MainMenu);
                    }
                    ExitDialog::Game => {
                        exit_events.send_default();
                    }
                },
                ExitDialogButton::Cancel => commands.entity(dialog_entity).despawn_recursive(),
            }
        }
    }

    fn close(mut commands: Commands, ingame_menus: Query<Entity, With<IngameMenu>>) {
        commands.entity(ingame_menus.single()).despawn_recursive();
    }
}

fn setup_exit_dialog(
    commands: &mut Commands,
    root_entity: Entity,
    theme: &Theme,
    exit_dialog: ExitDialog,
) {
    commands.entity(root_entity).with_children(|parent| {
        parent
            .spawn((exit_dialog, DialogBundle::new(theme)))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent.spawn(LabelBundle::normal(theme, exit_dialog.label()));

                        parent
                            .spawn(NodeBundle {
                                style: Style {
                                    column_gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| {
                                for button in ExitDialogButton::iter() {
                                    parent.spawn((
                                        button,
                                        TextButtonBundle::normal(theme, button.to_string()),
                                    ));
                                }
                            });
                    });
            });
    });
}

#[derive(Component)]
struct IngameMenu;

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum IngameMenuButton {
    Resume,
    Save,
    Settings,
    World,
    #[strum(serialize = "Main menu")]
    MainMenu,
    #[strum(serialize = "Exit game")]
    ExitGame,
}

#[derive(Component, Clone, Copy)]
enum ExitDialog {
    MainMenu,
    Game,
}

impl ExitDialog {
    fn label(&self) -> &'static str {
        match self {
            ExitDialog::MainMenu => "Are you sure you want to exit to the main menu?",
            ExitDialog::Game => "Are you sure you want to exit the game?",
        }
    }
}

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum ExitDialogButton {
    #[strum(serialize = "Save & exit")]
    SaveAndExit,
    Exit,
    Cancel,
}

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum SaveAsDialogButton {
    Save,
    Cancel,
}
