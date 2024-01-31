use bevy::{prelude::*, reflect::GetPath, ui::FocusPolicy};
use leafwing_input_manager::user_input::InputKind;
use strum::{Display, EnumIter, IntoEnumIterator};

use super::{
    theme::Theme,
    widget::{
        button::{ButtonText, ExclusiveButton, TabContent, TextButtonBundle, Toggled},
        checkbox::{Checkbox, CheckboxBundle},
        click::Click,
        ui_root::UiRoot,
        DialogBundle, LabelBundle,
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
        app.add_event::<SettingsMenuOpen>().add_systems(
            Update,
            (
                Self::setup_system.run_if(on_event::<SettingsMenuOpen>()),
                (
                    Self::mapping_button_text_system,
                    Self::mapping_button_system,
                    Self::binding_system.run_if(any_with_component::<BindingButton>()),
                    Self::binding_dialog_button_system,
                    Self::settings_button_system,
                )
                    .run_if(any_with_component::<SettingsMenu>()),
            ),
        );
    }
}

impl SettingsMenuPlugin {
    fn setup_system(
        mut commands: Commands,
        mut tab_commands: Commands,
        settings: Res<Settings>,
        theme: Res<Theme>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((
                    SettingsMenu,
                    Interaction::None,
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            flex_direction: FlexDirection::Column,
                            align_self: AlignSelf::Center,
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            padding: theme.padding.global,
                            ..Default::default()
                        },
                        focus_policy: FocusPolicy::Block,
                        background_color: theme.background_color.into(),
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    let tabs_entity = parent
                        .spawn(NodeBundle {
                            style: Style {
                                justify_content: JustifyContent::Center,
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .id();

                    for tab in SettingsTab::iter() {
                        let content_entity = parent
                            .spawn(NodeBundle {
                                style: Style {
                                    padding: theme.padding.normal,
                                    column_gap: theme.gap.normal,
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| match tab {
                                SettingsTab::Video => setup_video_tab(parent, &theme, &settings),
                                SettingsTab::Controls => {
                                    setup_controls_tab(parent, &theme, &settings)
                                }
                                SettingsTab::Developer => {
                                    setup_developer_tab(parent, &theme, &settings)
                                }
                            })
                            .id();

                        tab_commands
                            .spawn((
                                TabContent(content_entity),
                                ExclusiveButton,
                                Toggled(tab == Default::default()),
                                TextButtonBundle::normal(&theme, tab.to_string()),
                            ))
                            .set_parent(tabs_entity);
                    }

                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                align_items: AlignItems::End,
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                justify_content: JustifyContent::End,
                                column_gap: theme.gap.normal,
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
        });
    }

    fn mapping_button_text_system(
        mut buttons: Query<(&Mapping, &mut ButtonText), Changed<Mapping>>,
    ) {
        for (mapping, mut text) in &mut buttons {
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

    fn mapping_button_system(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        theme: Res<Theme>,
        roots: Query<Entity, With<UiRoot>>,
        buttons: Query<(Entity, &Mapping)>,
    ) {
        for event in click_events.read() {
            let Ok((entity, mapping)) = buttons.get(event.0) else {
                continue;
            };

            commands.entity(roots.single()).with_children(|parent| {
                parent
                    .spawn((BindingButton(entity), DialogBundle::new(&theme)))
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
                                parent.spawn((
                                    BindingLabel,
                                    LabelBundle::normal(
                                        &theme,
                                        format!("Binding \"{}\", press any key", mapping.action),
                                    ),
                                ));
                                parent
                                    .spawn(NodeBundle {
                                        style: Style {
                                            column_gap: theme.gap.normal,
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    })
                                    .with_children(|parent| {
                                        for dialog_button in BindingDialogButton::iter() {
                                            let mut button_bundle = TextButtonBundle::normal(
                                                &theme,
                                                dialog_button.to_string(),
                                            );
                                            if dialog_button == BindingDialogButton::Replace {
                                                button_bundle.button_bundle.style.display =
                                                    Display::None;
                                            }
                                            parent.spawn((dialog_button, button_bundle));
                                        }
                                    });
                            });
                    });
            });
        }
    }

    fn binding_system(
        mut commands: Commands,
        mut input_events: InputEvents,
        dialogs: Query<(Entity, &BindingButton)>,
        mut mapping_buttons: Query<(Entity, &mut Mapping)>,
        mut labels: Query<&mut Text, With<BindingLabel>>,
        mut dialog_buttons: Query<(&mut Style, &BindingDialogButton)>,
    ) {
        if let Some(input_kind) = input_events.input_kind() {
            let (dialog_entity, binding_button) = dialogs.single();
            if let Some((conflict_entity, mapping)) = mapping_buttons
                .iter()
                .find(|(_, mapping)| mapping.input_kind == Some(input_kind))
            {
                labels.single_mut().sections[0].value = format!(
                    "\"{input_kind}\" is already used by \"{:?}\"",
                    mapping.action
                );

                commands
                    .entity(dialog_entity)
                    .insert(ConflictButton(conflict_entity));

                let (mut style, _) = dialog_buttons
                    .iter_mut()
                    .find(|(_, &button)| button == BindingDialogButton::Replace)
                    .expect("replace button should be spawned with the dialog");
                style.display = Display::Flex;
            } else {
                let mut mapping = mapping_buttons
                    .get_component_mut::<Mapping>(binding_button.0)
                    .expect("binding dialog should point to a button with mapping");
                mapping.input_kind = Some(input_kind);
                commands.entity(dialog_entity).despawn_recursive();
            }
        }
    }

    fn binding_dialog_button_system(
        mut commands: Commands,
        mut click_events: EventReader<Click>,
        mut mapping_buttons: Query<&mut Mapping>,
        dialog_buttons: Query<&BindingDialogButton>,
        dialogs: Query<(Entity, Option<&ConflictButton>, &BindingButton)>,
    ) {
        for event in click_events.read() {
            if let Ok(dialog_button) = dialog_buttons.get(event.0) {
                let (entity, conflict_button, binding_button) = dialogs.single();
                match dialog_button {
                    BindingDialogButton::Replace => {
                        let conflict_button = conflict_button
                            .expect("replace button should be clickable only with conflict");
                        let mut conflict_mapping = mapping_buttons
                            .get_mut(conflict_button.0)
                            .expect("binding conflict should point to a button");
                        let input_kind = conflict_mapping.input_kind;
                        conflict_mapping.input_kind = None;

                        let mut mapping = mapping_buttons
                            .get_mut(binding_button.0)
                            .expect("binding should point to a button");
                        mapping.input_kind = input_kind;
                    }
                    BindingDialogButton::Delete => {
                        let mut mapping = mapping_buttons
                            .get_mut(binding_button.0)
                            .expect("binding should point to a button");
                        mapping.input_kind = None;
                    }
                    BindingDialogButton::Cancel => (),
                }
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn settings_button_system(
        mut commands: Commands,
        mut apply_events: EventWriter<SettingsApply>,
        mut click_events: EventReader<Click>,
        mut settings: ResMut<Settings>,
        settings_menus: Query<Entity, With<SettingsMenu>>,
        settings_buttons: Query<&SettingsButton>,
        mapping_buttons: Query<&Mapping>,
        checkboxes: Query<(&Checkbox, &SettingsField)>,
    ) {
        for event in click_events.read() {
            let Ok(&settings_button) = settings_buttons.get(event.0) else {
                continue;
            };

            if settings_button == SettingsButton::Ok {
                for (checkbox, field) in &checkboxes {
                    let field_value = settings
                        .path_mut::<bool>(field.0)
                        .expect("fields with checkboxes should be stored as bools");
                    *field_value = checkbox.0;
                }
                settings.controls.mappings.clear();
                for mapping in &mapping_buttons {
                    if let Some(input_kind) = mapping.input_kind {
                        settings
                            .controls
                            .mappings
                            .entry(mapping.action)
                            .or_default()
                            .push(input_kind);
                    }
                }
                apply_events.send_default();
            }

            commands.entity(settings_menus.single()).despawn_recursive()
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
                row_gap: theme.gap.normal,
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
    const INPUTS_PER_ACTION: usize = 3;
    parent
        .spawn(NodeBundle {
            style: Style {
                display: Display::Grid,
                column_gap: theme.gap.normal,
                row_gap: theme.gap.normal,
                grid_template_columns: vec![GridTrack::auto(); INPUTS_PER_ACTION + 1],
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for (&action, inputs) in &settings.controls.mappings {
                parent.spawn(TextBundle::from_section(
                    action.to_string(),
                    theme.label.normal.clone(),
                ));

                for index in 0..INPUTS_PER_ACTION {
                    parent.spawn((
                        Mapping {
                            action,
                            input_kind: inputs.get(index).cloned(),
                        },
                        TextButtonBundle::normal(theme, String::new()),
                    ));
                }
            }
        });
}

fn setup_developer_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                row_gap: theme.gap.normal,
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
                CheckboxBundle::new(theme, settings.developer.wireframe, "Wireframe"),
                setting_field!(settings.developer.wireframe),
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

// Creates a settings menu node.
#[derive(Default, Event)]
pub(super) struct SettingsMenuOpen;

#[derive(Component)]
struct SettingsMenu;

#[derive(Default, Display, EnumIter, PartialEq)]
enum SettingsTab {
    #[default]
    Video,
    Controls,
    Developer,
}

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum SettingsButton {
    Ok,
    Cancel,
}

#[derive(Clone, Component, Copy, Display, EnumIter, PartialEq)]
enum BindingDialogButton {
    Replace,
    Delete,
    Cancel,
}

/// Stores information about button mapping.
#[derive(Component)]
struct Mapping {
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
