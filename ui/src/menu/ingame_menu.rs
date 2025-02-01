use bevy::{app::AppExit, prelude::*};
use bevy_enhanced_input::prelude::*;
use project_harmonia_base::{
    core::GameState,
    game_world::{GameSave, WorldState},
};
use project_harmonia_widgets::{
    button::ButtonKind, dialog::Dialog, label::LabelKind, theme::Theme,
};

use super::settings_menu::SettingsMenuOpen;

pub(super) struct InGameMenuPlugin;

impl Plugin for InGameMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<IngameMenu>()
            .add_observer(toggle)
            .add_systems(OnEnter(WorldState::Family), setup)
            .add_systems(OnEnter(WorldState::City), setup);
    }
}

fn toggle(
    _trigger: Trigger<Started<ToggleIngameMenu>>,
    mut commands: Commands,
    menu: Single<(Entity, &Parent, &mut Node), With<IngameMenu>>,
) {
    let (entity, parent, mut node) = menu.into_inner();
    match node.display {
        Display::Flex => {
            info!("closing in-game menu");
            node.display = Display::None;
        }
        Display::None => {
            info!("showing in-game menu");
            node.display = Display::Flex;
        }
        Display::Block | Display::Grid => unreachable!(),
    }

    // Reparent to keep it on top of all UI.
    commands.entity(entity).set_parent(**parent);
}

fn setup(
    mut commands: Commands,
    world_state: Res<State<WorldState>>,
    theme: Res<Theme>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
) {
    commands.entity(*root_entity).with_children(|parent| {
        parent
            .spawn((
                IngameMenu,
                Node {
                    display: Display::None,
                    ..Default::default()
                },
                StateScoped(**world_state),
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        },
                        theme.panel_background,
                    ))
                    .with_children(|parent| {
                        parent.spawn((LabelKind::Normal, Text::new("Main menu")));

                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Resume"))
                            .observe(resume);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Save"))
                            .observe(save);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Settings"))
                            .observe(open_settings);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("World"))
                            .observe(open_world);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Main menu"))
                            .observe(exit_to_main_menu);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Exit game"))
                            .observe(exit_game);
                    });
            });
    });
}

fn resume(_trigger: Trigger<Pointer<Click>>, mut menu_node: Single<&mut Node, With<IngameMenu>>) {
    info!("closing in-game menu");
    menu_node.display = Display::None;
}

fn save(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut menu_node: Single<&mut Node, With<IngameMenu>>,
) {
    info!("closing in-game menu");
    commands.trigger(GameSave);
    menu_node.display = Display::None;
}

fn open_settings(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.trigger(SettingsMenuOpen);
}

fn open_world(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
    commands.set_state(WorldState::World);
}

fn exit_to_main_menu(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    theme: Res<Theme>,
    menu_entity: Single<Entity, With<IngameMenu>>,
) {
    commands.entity(*menu_entity).with_children(|parent| {
        setup_exit_dialog(parent, &theme, ExitDialog::MainMenu);
    });
}

fn exit_game(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    theme: Res<Theme>,
    menu_entity: Single<Entity, With<IngameMenu>>,
) {
    commands.entity(*menu_entity).with_children(|parent| {
        setup_exit_dialog(parent, &theme, ExitDialog::Game);
    });
}

fn setup_exit_dialog(parent: &mut ChildBuilder, theme: &Theme, exit_dialog: ExitDialog) {
    info!("showing exit dialog");
    parent.spawn(exit_dialog).with_children(|parent| {
        parent
            .spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: theme.padding.normal,
                    row_gap: theme.gap.normal,
                    ..Default::default()
                },
                theme.panel_background,
            ))
            .with_children(|parent| {
                parent.spawn((LabelKind::Normal, Text::new(exit_dialog.label())));

                parent
                    .spawn(Node {
                        column_gap: theme.gap.normal,
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Save & exit"))
                            .observe(save_and_exit);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Exit"))
                            .observe(exit_without_saving);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Cancel"))
                            .observe(cancel_exit);
                    });
            });
    });
}

fn save_and_exit(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut exit_events: EventWriter<AppExit>,
    exit_dialog: Single<&ExitDialog>,
) {
    commands.trigger(GameSave);
    match *exit_dialog {
        ExitDialog::MainMenu => commands.set_state(GameState::Menu),
        ExitDialog::Game => {
            info!("exiting game");
            exit_events.send_default();
        }
    }
}

fn exit_without_saving(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut exit_events: EventWriter<AppExit>,
    exit_dialog: Single<&ExitDialog>,
) {
    match *exit_dialog {
        ExitDialog::MainMenu => commands.set_state(GameState::Menu),
        ExitDialog::Game => {
            info!("exiting game");
            exit_events.send_default();
        }
    }
}

fn cancel_exit(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    dialog_entity: Single<Entity, With<ExitDialog>>,
) {
    info!("cancelling exit");
    commands.entity(*dialog_entity).despawn_recursive();
}

#[derive(Component)]
#[require(Name(|| Name::new("Ingame menu")), Dialog)]
struct IngameMenu;

impl InputContext for IngameMenu {
    const PRIORITY: isize = -1;

    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        ctx.bind::<ToggleIngameMenu>()
            .to((KeyCode::Escape, GamepadButton::Start));
        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct ToggleIngameMenu;

#[derive(Component, Clone, Copy)]
#[require(Name(|| Name::new("Exit dialog")), Dialog)]
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
