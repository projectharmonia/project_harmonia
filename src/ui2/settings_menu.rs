use bevy::prelude::*;
use leafwing_input_manager::{
    user_input::{InputKind, UserInput},
    Actionlike,
};
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    theme::Theme,
    ui_state::UiState,
    widget::{
        button::{ButtonText, ExclusiveButton, Pressed, TextButtonBundle},
        checkbox::CheckboxBundle,
        ui_root::UiRoot,
        LabelBundle, Modal, ModalBundle,
    },
};
use crate::core::{
    action::Action,
    input_events::InputEvents,
    settings::{Settings, SettingsApply},
};

pub(super) struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::setup_system.in_schedule(OnEnter(UiState::Settings)))
            .add_systems(
                (
                    Self::binding_button_system,
                    Self::tab_display_system,
                    Self::binding_dialog_system,
                    Self::binding_confirmation_system
                        .run_if(not(any_with_component::<ConflictButton>()))
                        .run_if(any_with_component::<BindingButton>()),
                    Self::conflict_button_system,
                    Self::settings_buttons_system,
                )
                    .in_set(OnUpdate(UiState::Settings)),
            );
    }
}

impl SettingsMenuPlugin {
    fn setup_system(mut commands: Commands, settings: Res<Settings>, theme: Res<Theme>) {
        commands
            .spawn((
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        size: Size::all(Val::Percent(100.0)),
                        padding: theme.global_padding,
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
                            justify_content: JustifyContent::Center,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        for (index, tab) in SettingsTab::iter().enumerate() {
                            parent.spawn((
                                tab,
                                ExclusiveButton,
                                Pressed(index == 0),
                                TextButtonBundle::normal(&theme, tab.to_string()),
                            ));
                        }
                    });

                for tab in SettingsTab::iter() {
                    parent
                        .spawn((tab, NodeBundle::default()))
                        .with_children(|parent| match tab {
                            SettingsTab::Video => setup_video_tab(parent, &theme, &settings),
                            SettingsTab::Controls => setup_controls_tab(parent, &theme, &settings),
                            SettingsTab::Developer => {
                                setup_developer_tab(parent, &theme, &settings)
                            }
                        });
                }

                parent
                    .spawn(NodeBundle {
                        style: Style {
                            align_items: AlignItems::End,
                            size: Size::all(Val::Percent(100.0)),
                            justify_content: JustifyContent::End,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        for button in SettingsButton::iter() {
                            parent.spawn((
                                button,
                                TextButtonBundle::normal(&theme, button.to_string()),
                            ));
                        }
                    });
            });
    }

    fn binding_button_system(mut modals: Query<(&Binding, &mut ButtonText), Changed<Binding>>) {
        for (binding, mut text) in &mut modals {
            text.0 = match binding.input_kind {
                Some(InputKind::GamepadButton(gamepad_button)) => {
                    format!("{gamepad_button:?}")
                }
                Some(InputKind::Keyboard(keycode)) => {
                    format!("{keycode:?}")
                }
                Some(InputKind::Mouse(mouse_button)) => {
                    format!("{mouse_button:?}")
                }
                _ => "Empty".to_string(),
            };
        }
    }

    fn tab_display_system(
        tabs: Query<(&Pressed, &SettingsTab), Changed<Pressed>>,
        mut tab_nodes: Query<(&mut Style, &SettingsTab), Without<Pressed>>,
    ) {
        for (pressed, tab) in &tabs {
            let (mut style, _) = tab_nodes
                .iter_mut()
                .find(|&(_, node_tab)| node_tab == tab)
                .expect("tabs should have associated nodes");
            style.display = if pressed.0 {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    fn binding_dialog_system(
        mut commands: Commands,
        theme: Res<Theme>,
        roots: Query<Entity, With<UiRoot>>,
        buttons: Query<(Entity, &Interaction), (Changed<Interaction>, With<Binding>)>,
    ) {
        for (entity, &interaction) in &buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            commands.entity(roots.single()).with_children(|parent| {
                parent
                    .spawn(ModalBundle::new(&theme))
                    .with_children(|parent| {
                        parent
                            .spawn((
                                BindingButton(entity),
                                NodeBundle {
                                    style: theme.element.binding_dialog.clone(),
                                    background_color: theme.modal.panel_color.into(),
                                    ..Default::default()
                                },
                            ))
                            .with_children(|parent| {
                                parent.spawn((
                                    BindingLabel,
                                    LabelBundle::new(&theme, "Press any key"),
                                ));
                            });
                    });
            });
        }
    }

    fn binding_confirmation_system(
        mut commands: Commands,
        mut input_events: InputEvents,
        settings: Res<Settings>,
        theme: Res<Theme>,
        dialogs: Query<(Entity, &BindingButton)>,
        modals: Query<Entity, With<Modal>>,
        mut buttons: Query<&mut Binding>,
        mut binding_labels: Query<&mut Text, With<BindingLabel>>,
    ) {
        if let Some(input_kind) = input_events.input_kind() {
            let (entity, binding_button) = dialogs.single();
            let mut binding = buttons
                .get_mut(binding_button.0)
                .expect("binding dialog should point to a button with binding");

            if let Some((_, conflict_action)) = settings
                .controls
                .mappings
                .iter()
                .filter(|&(_, action)| action != binding.action)
                .find(|(inputs, _)| inputs.contains(&input_kind.into()))
            {
                let mut text = binding_labels.single_mut();
                text.sections[0].value =
                    format!("Input \"{input_kind}\" is already used by \"{conflict_action:?}\"",);

                commands
                    .entity(entity)
                    .insert(BindingConflict(input_kind))
                    .with_children(|parent| {
                        parent.spawn(NodeBundle::default()).with_children(|parent| {
                            for button in ConflictButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(&theme, button.to_string()),
                                ));
                            }
                        });
                    });
            } else {
                binding.input_kind = Some(input_kind);
                commands.entity(modals.single()).despawn_recursive();
            }
        }
    }

    fn conflict_button_system(
        mut commands: Commands,
        conflict_buttons: Query<(&Interaction, &ConflictButton), Changed<Interaction>>,
        dialogs: Query<(&BindingConflict, &BindingButton)>,
        mut binding_buttons: Query<&mut Binding>,
        modals: Query<Entity, With<Modal>>,
    ) {
        for (&interaction, conflict_button) in &conflict_buttons {
            if interaction == Interaction::Clicked {
                let (conflict, binding_button) = dialogs.single();
                match conflict_button {
                    ConflictButton::Replace => {
                        let mut conflict_binding = binding_buttons
                            .iter_mut()
                            .find(|binding| binding.input_kind == Some(conflict.0))
                            .expect("binding with the same input should exist on conflict");
                        conflict_binding.input_kind = None;

                        let mut binding = binding_buttons
                            .get_mut(binding_button.0)
                            .expect("binding should point to a button");
                        binding.input_kind = Some(conflict.0);
                        commands.entity(modals.single()).despawn_recursive();
                    }
                    ConflictButton::Cancel => commands.entity(modals.single()).despawn_recursive(),
                }
            }
        }
    }

    fn settings_buttons_system(
        mut apply_events: EventWriter<SettingsApply>,
        mut ui_state: ResMut<NextState<UiState>>,
        buttons: Query<(&Interaction, &SettingsButton), Changed<Interaction>>,
    ) {
        for (&interaction, &button) in &buttons {
            if interaction == Interaction::Clicked {
                match button {
                    SettingsButton::Ok => {
                        apply_events.send_default();
                        ui_state.set(UiState::MainMenu);
                    }
                    SettingsButton::Cancel => ui_state.set(UiState::MainMenu),
                }
            }
        }
    }
}

fn setup_video_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(CheckboxBundle::new(
                theme,
                settings.video.perf_stats,
                "Display performance stats",
            ));
        });
}

fn setup_controls_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    // TODO 0.11: Use grid layout.
    const PADDING: f32 = 7.5;
    parent
        .spawn(NodeBundle {
            style: Style {
                gap: Size::all(Val::Px(PADDING * 2.0)),
                padding: UiRect::all(Val::Px(PADDING)),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for action in Action::variants() {
                parent.spawn(TextBundle::from_section(
                    action.to_string(),
                    theme.text.normal.clone(),
                ));
            }
        });

    for index in 0..3 {
        parent
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                for action in Action::variants() {
                    let inputs = settings.controls.mappings.get(action);
                    let input = inputs.get_at(index).cloned();
                    let input_kind = if let Some(UserInput::Single(input_kind)) = input {
                        Some(input_kind)
                    } else {
                        None
                    };
                    parent.spawn((
                        Binding { action, input_kind },
                        TextButtonBundle::normal(theme, String::new()),
                    ));
                }
            });
    }
}

fn setup_developer_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(CheckboxBundle::new(
                theme,
                settings.developer.game_inspector,
                "Enable game inspector",
            ));
            parent.spawn(CheckboxBundle::new(
                theme,
                settings.developer.debug_collisions,
                "Debug collisions",
            ));
            parent.spawn(CheckboxBundle::new(
                theme,
                settings.developer.debug_paths,
                "Debug navigation paths",
            ));
        });
}

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum SettingsTab {
    Video,
    Controls,
    Developer,
}

#[derive(Clone, Component, Copy, Display, EnumIter)]
enum SettingsButton {
    Ok,
    Cancel,
}

#[derive(Clone, Component, Copy, Display, EnumIter)]
enum ConflictButton {
    Replace,
    Cancel,
}

#[derive(Component)]
struct Binding {
    action: Action,
    input_kind: Option<InputKind>,
}

#[derive(Component)]
struct BindingButton(Entity);

#[derive(Component)]
struct BindingConflict(InputKind);

#[derive(Component)]
struct BindingLabel;
