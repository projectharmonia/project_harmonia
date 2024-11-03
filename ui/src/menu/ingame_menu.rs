use bevy::{app::AppExit, prelude::*};
use bevy_enhanced_input::prelude::*;
use project_harmonia_base::{
    core::GameState,
    game_world::{GameSave, WorldState},
};
use project_harmonia_widgets::{
    button::TextButtonBundle, click::Click, dialog::DialogBundle, label::LabelBundle, theme::Theme,
};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::settings_menu::SettingsMenuOpen;

pub(super) struct InGameMenuPlugin;

impl Plugin for InGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<IngameMenu>()
            .observe(Self::toggle)
            .add_systems(OnEnter(WorldState::Family), Self::setup)
            .add_systems(OnEnter(WorldState::City), Self::setup)
            .add_systems(
                Update,
                (Self::handle_menu_clicks, Self::handle_exit_dialog_clicks)
                    .run_if(any_with_component::<IngameMenu>),
            );
    }
}

impl InGameMenuPlugin {
    fn setup(
        mut commands: Commands,
        world_state: Res<State<WorldState>>,
        theme: Res<Theme>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((
                    IngameMenu,
                    DialogBundle::new(&theme).with_display(Display::None),
                    StateScoped(**world_state),
                ))
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

    fn toggle(
        _trigger: Trigger<Started<ToggleIngameMenu>>,
        mut menus: Query<&mut Style, With<IngameMenu>>,
    ) {
        let mut style = menus.single_mut();
        match style.display {
            Display::Flex => {
                info!("closing in-game menu");
                style.display = Display::None;
            }
            Display::None => {
                info!("showing in-game menu");
                style.display = Display::Flex;
            }
            Display::Block | Display::Grid => unreachable!(),
        }
    }

    fn handle_menu_clicks(
        mut commands: Commands,
        mut save_events: EventWriter<GameSave>,
        mut settings_events: EventWriter<SettingsMenuOpen>,
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        mut world_state: ResMut<NextState<WorldState>>,
        buttons: Query<&IngameMenuButton>,
        mut menus: Query<(Entity, &mut Style), With<IngameMenu>>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                IngameMenuButton::Resume => {
                    info!("closing in-game menu");
                    let (_, mut style) = menus.single_mut();
                    style.display = Display::None;
                }
                IngameMenuButton::Save => {
                    save_events.send_default();
                    info!("closing in-game menu");
                    let (_, mut style) = menus.single_mut();
                    style.display = Display::None;
                }
                IngameMenuButton::Settings => {
                    settings_events.send_default();
                }
                IngameMenuButton::World => world_state.set(WorldState::World),
                IngameMenuButton::MainMenu => {
                    let (entity, _) = menus.single();
                    commands.entity(entity).with_children(|parent| {
                        setup_exit_dialog(parent, &theme, ExitDialog::MainMenu);
                    });
                }
                IngameMenuButton::ExitGame => {
                    let (entity, _) = menus.single();
                    commands.entity(entity).with_children(|parent| {
                        setup_exit_dialog(parent, &theme, ExitDialog::Game);
                    });
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
                        ExitDialog::MainMenu => game_state.set(GameState::Menu),
                        ExitDialog::Game => {
                            info!("exiting game");
                            exit_events.send_default();
                        }
                    }
                }
                ExitDialogButton::Exit => match exit_dialog {
                    ExitDialog::MainMenu => game_state.set(GameState::Menu),
                    ExitDialog::Game => {
                        info!("exiting game");
                        exit_events.send_default();
                    }
                },
                ExitDialogButton::Cancel => {
                    info!("cancelling exit");
                    commands.entity(dialog_entity).despawn_recursive();
                }
            }
        }
    }
}

fn setup_exit_dialog(parent: &mut ChildBuilder, theme: &Theme, exit_dialog: ExitDialog) {
    info!("showing exit dialog");
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
}

#[derive(Component)]
struct IngameMenu;

impl InputContext for IngameMenu {
    const PRIORITY: isize = -1;

    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        ctx.bind::<ToggleIngameMenu>()
            .with(KeyCode::Escape)
            .with(GamepadButtonType::Start);
        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(dim = Bool)]
struct ToggleIngameMenu;

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
