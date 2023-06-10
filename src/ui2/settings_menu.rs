use bevy::{prelude::*, reflect::GetPath};
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
        checkbox::{Checkbox, CheckboxBundle},
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
                    Self::mapping_button_system,
                    Self::tab_display_system,
                    Self::binding_dialog_system,
                    Self::binding_confirmation_system
                        .run_if(any_with_component::<BindingButton>())
                        .run_if(not(any_with_component::<ConflictDialogButton>())),
                    Self::conflict_dialog_system,
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

    fn mapping_button_system(mut modals: Query<(&Mapping, &mut ButtonText), Changed<Mapping>>) {
        for (mapping, mut text) in &mut modals {
            text.0 = match mapping.input_kind {
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
        buttons: Query<(Entity, &Interaction), (Changed<Interaction>, With<Mapping>)>,
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
        theme: Res<Theme>,
        dialogs: Query<(Entity, &BindingButton)>,
        modals: Query<Entity, With<Modal>>,
        mut mapping_buttons: Query<(Entity, &mut Mapping)>,
        mut binding_labels: Query<&mut Text, With<BindingLabel>>,
    ) {
        if let Some(input_kind) = input_events.input_kind() {
            let (dialog_entity, binding_button) = dialogs.single();
            if let Some((conflict_entity, mapping)) = mapping_buttons
                .iter()
                .find(|(_, mapping)| mapping.input_kind == Some(input_kind))
            {
                let mut text = binding_labels.single_mut();
                text.sections[0].value = format!(
                    "Input \"{input_kind}\" is already used by \"{:?}\"",
                    mapping.action
                );

                commands
                    .entity(dialog_entity)
                    .insert(ConflictButton(conflict_entity))
                    .with_children(|parent| {
                        parent.spawn(NodeBundle::default()).with_children(|parent| {
                            for button in ConflictDialogButton::iter() {
                                parent.spawn((
                                    button,
                                    TextButtonBundle::normal(&theme, button.to_string()),
                                ));
                            }
                        });
                    });
            } else {
                let mut mapping = mapping_buttons
                    .get_component_mut::<Mapping>(binding_button.0)
                    .expect("binding dialog should point to a button with mapping");
                mapping.input_kind = Some(input_kind);
                commands.entity(modals.single()).despawn_recursive();
            }
        }
    }

    fn conflict_dialog_system(
        mut commands: Commands,
        conflict_buttons: Query<(&Interaction, &ConflictDialogButton), Changed<Interaction>>,
        dialogs: Query<(&ConflictButton, &BindingButton)>,
        mut mapping_buttons: Query<&mut Mapping>,
        modals: Query<Entity, With<Modal>>,
    ) {
        for (&interaction, dialog_button) in &conflict_buttons {
            if interaction == Interaction::Clicked {
                let (conflict_button, binding_button) = dialogs.single();
                match dialog_button {
                    ConflictDialogButton::Replace => {
                        let mut conflict_mapping = mapping_buttons
                            .get_mut(conflict_button.0)
                            .expect("binding conflict should point to a button");
                        let input_kind = conflict_mapping.input_kind;
                        conflict_mapping.input_kind = None;

                        let mut mapping = mapping_buttons
                            .get_mut(binding_button.0)
                            .expect("binding should point to a button");
                        mapping.input_kind = input_kind;
                        commands.entity(modals.single()).despawn_recursive();
                    }
                    ConflictDialogButton::Cancel => {
                        commands.entity(modals.single()).despawn_recursive()
                    }
                }
            }
        }
    }

    fn settings_buttons_system(
        mut apply_events: EventWriter<SettingsApply>,
        mut settings: ResMut<Settings>,
        mut ui_state: ResMut<NextState<UiState>>,
        buttons: Query<(&Interaction, &SettingsButton), Changed<Interaction>>,
        mapping_buttons: Query<&Mapping>,
        checkboxes: Query<(&Checkbox, &SettingsField)>,
    ) {
        for (&interaction, &button) in &buttons {
            if interaction != Interaction::Clicked {
                continue;
            }

            match button {
                SettingsButton::Ok => {
                    for (checkbox, field) in &checkboxes {
                        let field_value = settings
                            .path_mut::<bool>(field.0)
                            .expect("fields with checkboxes should be stored as bools");
                        *field_value = checkbox.0;
                    }
                    for mapping in &mapping_buttons {
                        if let Some(input_kind) = mapping.input_kind {
                            settings.controls.mappings.insert_at(
                                input_kind,
                                mapping.action,
                                mapping.index,
                            );
                        } else {
                            settings
                                .controls
                                .mappings
                                .remove_at(mapping.action, mapping.index);
                        }
                    }
                    apply_events.send_default();
                    ui_state.set(UiState::MainMenu);
                }
                SettingsButton::Cancel => ui_state.set(UiState::MainMenu),
            }
        }
    }
}

/// Creates [`SettingsField`] from passed field.
macro_rules! setting_field {
    ($path:expr) => {{
        let _validate_field = $path;
        SettingsField(stringify!($path).split_once('.').unwrap().1)
    }};
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
            parent.spawn((
                CheckboxBundle::new(
                    theme,
                    settings.video.perf_stats,
                    "Display performance stats",
                ),
                setting_field!(settings.video.perf_stats),
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
                        Mapping {
                            action,
                            input_kind,
                            index,
                        },
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
            parent.spawn((
                CheckboxBundle::new(
                    theme,
                    settings.developer.debug_collisions,
                    "Debug collisions",
                ),
                setting_field!(settings.developer.debug_collisions),
            ));
            parent.spawn((
                CheckboxBundle::new(
                    theme,
                    settings.developer.debug_paths,
                    "Debug navigation paths",
                ),
                setting_field!(settings.developer.debug_paths),
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
enum ConflictDialogButton {
    Replace,
    Cancel,
}

/// Stores information about button mapping.
#[derive(Component)]
struct Mapping {
    index: usize,
    action: Action,
    input_kind: Option<InputKind>,
}

/// Contains button entity that was selected for binding.
#[derive(Component)]
struct BindingButton(Entity);

/// Contains button entity that has the same `input_kind` as the [`BindingButton`].
#[derive(Component)]
struct ConflictButton(Entity);

/// Marker for label with binding dialog text.
#[derive(Component)]
struct BindingLabel;

#[derive(Component)]
struct SettingsField(&'static str);
