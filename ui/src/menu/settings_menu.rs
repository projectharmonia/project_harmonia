use std::fmt::Write;

use bevy::{
    input::{common_conditions::*, keyboard::KeyboardInput, mouse::MouseButtonInput, ButtonState},
    prelude::*,
    reflect::GetPath,
};
use bevy_enhanced_input::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use project_harmonia_base::settings::{
    DeveloperSettings, KeyboardSettings, Settings, SettingsApply, VideoSettings,
};
use project_harmonia_widgets::{
    button::{ButtonKind, TabContent, Toggled},
    checkbox::Checkbox,
    dialog::Dialog,
    label::LabelKind,
    theme::Theme,
};

pub(super) struct SettingsMenuPlugin;

impl Plugin for SettingsMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(setup).add_systems(
            Update,
            (
                update_button_text,
                (
                    cancel_binding
                        .never_param_warn()
                        .run_if(input_just_pressed(KeyCode::Escape)),
                    bind.never_param_warn(),
                )
                    .chain(),
            )
                .run_if(any_with_component::<SettingsMenu>),
        );
    }
}

fn setup(
    _trigger: Trigger<SettingsMenuOpen>,
    mut commands: Commands,
    mut tab_commands: Commands,
    settings: Res<Settings>,
    theme: Res<Theme>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
) {
    info!("opening setting menu");
    commands.entity(*root_entity).with_children(|parent| {
        parent
            .spawn((
                SettingsMenu,
                Node {
                    position_type: PositionType::Absolute,
                    flex_direction: FlexDirection::Column,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    padding: theme.padding.global,
                    row_gap: theme.gap.normal,
                    ..Default::default()
                },
                theme.background_color,
            ))
            .with_children(|parent| {
                let tabs_entity = parent
                    .spawn(Node {
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    })
                    .id();

                for tab in SettingsTab::iter() {
                    let content_entity = match tab {
                        SettingsTab::Video => setup_video_tab(parent, &theme, &settings.video),
                        SettingsTab::Keyboard => {
                            setup_keyboard_tab(parent, &theme, &settings.keyboard)
                        }
                        SettingsTab::Developer => {
                            setup_developer_tab(parent, &theme, &settings.developer)
                        }
                    };

                    tab_commands
                        .spawn((
                            ButtonKind::Normal,
                            TabContent(content_entity),
                            Toggled(tab == Default::default()),
                        ))
                        .with_child(Text::new(tab.text()))
                        .set_parent(tabs_entity);
                }

                parent
                    .spawn(Node {
                        align_items: AlignItems::End,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        justify_content: JustifyContent::End,
                        column_gap: theme.gap.normal,
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Ok"))
                            .observe(confirm);
                        parent
                            .spawn(ButtonKind::Normal)
                            .with_child(Text::new("Cancel"))
                            .observe(cancel);
                    });
            });
    });
}

/// Creates [`SettingsField`] from passed field.
macro_rules! settings_field {
    ($field:ident . $($rest:ident).+) => {{
        let _validate_field = Settings::default().$field.$($rest).+;
        SettingsField(stringify!($path))
    }};
}

/// Stores name of the [`Settings`] field.
///
/// Used to utilize reflection when applying settings.
#[derive(Component, Clone, Copy)]
struct SettingsField(&'static str);

fn setup_video_tab(parent: &mut ChildBuilder, theme: &Theme, video: &VideoSettings) -> Entity {
    parent
        .spawn(Node {
            padding: theme.padding.normal,
            row_gap: theme.gap.normal,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Checkbox(video.fullscreen),
                    settings_field!(video.fullscreen),
                ))
                .with_child(Text::new("Fullscreen"));
        })
        .id()
}

/// Number of input columns.
const INPUTS_PER_ACTION: usize = 3;

fn setup_keyboard_tab(
    parent: &mut ChildBuilder,
    theme: &Theme,
    keyboard: &KeyboardSettings,
) -> Entity {
    parent
        .spawn(Node {
            display: Display::Grid,
            column_gap: theme.gap.normal,
            row_gap: theme.gap.normal,
            grid_template_columns: vec![GridTrack::auto(); INPUTS_PER_ACTION + 1],
            ..Default::default()
        })
        .with_children(|parent| {
            setup_action_row(
                parent,
                theme,
                "Camera forward",
                &keyboard.camera_forward,
                settings_field!(keyboard.camera_forward),
            );
            setup_action_row(
                parent,
                theme,
                "Camera left",
                &keyboard.camera_left,
                settings_field!(keyboard.camera_left),
            );
            setup_action_row(
                parent,
                theme,
                "Camera backward",
                &keyboard.camera_backward,
                settings_field!(keyboard.camera_backward),
            );
            setup_action_row(
                parent,
                theme,
                "Camera right",
                &keyboard.camera_right,
                settings_field!(keyboard.camera_right),
            );
            setup_action_row(
                parent,
                theme,
                "Rotate left",
                &keyboard.rotate_left,
                settings_field!(keyboard.rotate_left),
            );
            setup_action_row(
                parent,
                theme,
                "Rotate right",
                &keyboard.rotate_right,
                settings_field!(keyboard.rotate_right),
            );
            setup_action_row(
                parent,
                theme,
                "Zoom in",
                &keyboard.zoom_in,
                settings_field!(keyboard.zoom_in),
            );
            setup_action_row(
                parent,
                theme,
                "Zoom out",
                &keyboard.zoom_out,
                settings_field!(keyboard.zoom_out),
            );
            setup_action_row(
                parent,
                theme,
                "Delete object",
                &keyboard.delete,
                settings_field!(keyboard.delete),
            );
            setup_action_row(
                parent,
                theme,
                "Free placement",
                &keyboard.free_placement,
                settings_field!(keyboard.free_placement),
            );
            setup_action_row(
                parent,
                theme,
                "Ordinal placement",
                &keyboard.ordinal_placement,
                settings_field!(keyboard.ordinal_placement),
            );
        })
        .id()
}

fn setup_action_row(
    parent: &mut ChildBuilder,
    theme: &Theme,
    name: &'static str,
    inputs: &[Input],
    field: SettingsField,
) {
    parent.spawn((LabelKind::Normal, Text::new(name)));
    for index in 0..INPUTS_PER_ACTION {
        parent
            .spawn(Node {
                column_gap: theme.gap.normal,
                align_items: AlignItems::Center,
                ..Default::default()
            })
            .with_children(|parent| {
                let button_entity = parent
                    .spawn((
                        field,
                        Name::new(name),
                        InputButton {
                            input: inputs.get(index).copied(),
                        },
                    ))
                    .with_child(Text::default()) // Will be updated automatically on `InputButton` insertion
                    .observe(show_binding_dialog)
                    .id();
                parent
                    .spawn(DeleteButton { button_entity })
                    .with_child(Text::new("X"))
                    .observe(delete_binding);
            });
    }
}

fn delete_binding(
    trigger: Trigger<Pointer<Click>>,
    mut input_buttons: Query<(&Name, &mut InputButton)>,
    delete_buttons: Query<&DeleteButton>,
) {
    let delete_button = delete_buttons.get(trigger.entity()).unwrap();
    let (name, mut input_button) = input_buttons
        .get_mut(delete_button.button_entity)
        .expect("delete button should point to an input button");
    info!("deleting binding for '{name}'");
    input_button.input = None;
}

fn show_binding_dialog(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    theme: Res<Theme>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    names: Query<&Name>,
) {
    let name = names.get(trigger.entity()).unwrap();
    info!("starting binding for '{name}'");

    commands.entity(*root_entity).with_children(|parent| {
        parent
            .spawn(BindingDialog {
                button_entity: trigger.entity(),
            })
            .with_children(|parent| {
                parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        },
                        theme.panel_background,
                    ))
                    .with_children(|parent| {
                        parent.spawn((
                            LabelKind::Normal,
                            TextLayout {
                                justify: JustifyText::Center,
                                ..Default::default()
                            },
                            Text::new(format!(
                                "Binding \"{name}\", \npress any key or Esc to cancel",
                            )),
                        ));
                    });
            });
    });
}

fn bind(
    mut commands: Commands,
    mut key_events: EventReader<KeyboardInput>,
    mut mouse_button_events: EventReader<MouseButtonInput>,
    theme: Res<Theme>,
    dialog: Single<(Entity, &BindingDialog)>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    mut buttons: Query<(Entity, &Name, &mut InputButton)>,
) {
    let keys = key_events
        .read()
        .filter(|event| event.state == ButtonState::Pressed)
        .map(|event| event.key_code.into());
    let mouse_buttons = mouse_button_events
        .read()
        .filter(|event| event.state == ButtonState::Pressed)
        .map(|event| event.button.into());

    let Some(input) = keys.chain(mouse_buttons).next() else {
        return;
    };

    let (dialog_entity, dialog) = *dialog;

    if let Some((conflict_entity, name, _)) = buttons
        .iter()
        .find(|(.., button)| button.input == Some(input))
    {
        info!("found conflict with '{name}' for '{input}'");

        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn(ConflictDialog {
                    button_entity: dialog.button_entity,
                    conflict_entity,
                })
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Center,
                                padding: theme.padding.normal,
                                row_gap: theme.gap.normal,
                                ..Default::default()
                            },
                            theme.panel_background,
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                LabelKind::Normal,
                                Text::new(format!("\"{input}\" is already used by \"{name}\"",)),
                            ));
                            parent
                                .spawn(Node {
                                    column_gap: theme.gap.normal,
                                    ..Default::default()
                                })
                                .with_children(|parent| {
                                    parent
                                        .spawn(ButtonKind::Normal)
                                        .with_child(Text::new("Replace"))
                                        .observe(replace_binding);
                                    parent
                                        .spawn(ButtonKind::Normal)
                                        .with_child(Text::new("Cancel"))
                                        .observe(cancel_replace_binding);
                                });
                        });
                });
        });
    } else {
        let (_, name, mut button) = buttons
            .get_mut(dialog.button_entity)
            .expect("binding dialog should point to a button with input");
        info!("assigning '{input}' to '{name}'");
        button.input = Some(input);
    }

    commands.entity(dialog_entity).despawn_recursive();
}

fn cancel_binding(mut commands: Commands, dialog_entity: Single<Entity, With<BindingDialog>>) {
    info!("cancelling binding");
    commands.entity(*dialog_entity).despawn_recursive();
}

fn replace_binding(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    dialog: Single<(Entity, &ConflictDialog)>,
    mut buttons: Query<(&Name, &mut InputButton)>,
) {
    let (dialog_entity, dialog) = *dialog;
    let (_, mut conflict_button) = buttons
        .get_mut(dialog.conflict_entity)
        .expect("binding conflict should point to a button");
    let input = conflict_button.input;
    conflict_button.input = None;

    let (name, mut button) = buttons
        .get_mut(dialog.button_entity)
        .expect("binding should point to a button");
    button.input = input;

    info!("reassigning binding to '{name}'");
    commands.entity(dialog_entity).despawn_recursive();
}

fn cancel_replace_binding(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    dialog_entity: Single<Entity, With<ConflictDialog>>,
) {
    info!("cancelling replace binding");
    commands.entity(*dialog_entity).despawn_recursive();
}

fn setup_developer_tab(
    parent: &mut ChildBuilder,
    theme: &Theme,
    developer: &DeveloperSettings,
) -> Entity {
    parent
        .spawn(Node {
            padding: theme.padding.normal,
            row_gap: theme.gap.normal,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Checkbox(developer.free_camera_rotation),
                    settings_field!(developer.free_camera_rotation),
                ))
                .with_child(Text::new("Free camera rotation"));
            parent
                .spawn((
                    Checkbox(developer.wireframe),
                    settings_field!(developer.wireframe),
                ))
                .with_child(Text::new("Display wireframe"));
            parent
                .spawn((
                    Checkbox(developer.colliders),
                    settings_field!(developer.colliders),
                ))
                .with_child(Text::new("Display colliders"));
            parent
                .spawn((Checkbox(developer.paths), settings_field!(developer.paths)))
                .with_child(Text::new("Display navigation paths"));
            parent
                .spawn((
                    Checkbox(developer.nav_mesh),
                    settings_field!(developer.nav_mesh),
                ))
                .with_child(Text::new("Display navigation mesh"));
        })
        .id()
}

fn confirm(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    mut settings: ResMut<Settings>,
    menu_entity: Single<Entity, With<SettingsMenu>>,
    buttons: Query<(&InputButton, &SettingsField)>,
    checkboxes: Query<(&Checkbox, &SettingsField)>,
) {
    info!("confirming settings");

    for (checkbox, field) in &checkboxes {
        let field_value = settings
            .path_mut::<bool>(field.0)
            .expect("fields with checkboxes should be stored as bools");
        *field_value = checkbox.0;
    }
    settings.keyboard.clear();
    for (button, field) in &buttons {
        if let Some(input) = button.input {
            let field_value = settings
                .path_mut::<Vec<Input>>(field.0)
                .expect("fields with mappings should be stored as Vec");
            field_value.push(input);
        }
    }

    commands.trigger(SettingsApply);
    commands.entity(*menu_entity).despawn_recursive();
}

fn cancel(
    _trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    menu_entity: Single<Entity, With<SettingsMenu>>,
) {
    info!("closing setting menu");
    commands.entity(*menu_entity).despawn_recursive()
}

fn update_button_text(
    buttons: Query<(&InputButton, &Children), Changed<InputButton>>,
    mut text: Query<&mut Text>,
) {
    for (button, children) in &buttons {
        let mut iter = text.iter_many_mut(children);
        let mut text = iter.fetch_next().unwrap();
        text.clear();
        if let Some(input) = button.input {
            write!(text, "{input}").unwrap();
        } else {
            write!(text, "Empty").unwrap();
        };
    }
}

// Creates a settings menu node.
#[derive(Event)]
pub(super) struct SettingsMenuOpen;

#[derive(Component)]
struct SettingsMenu;

#[derive(Default, EnumIter, PartialEq, Clone, Copy)]
enum SettingsTab {
    #[default]
    Video,
    Keyboard,
    Developer,
}

impl SettingsTab {
    fn text(self) -> &'static str {
        match self {
            SettingsTab::Video => "Video",
            SettingsTab::Keyboard => "Keyboard",
            SettingsTab::Developer => "Developer",
        }
    }
}

/// Stores information about button mapping.
#[derive(Component)]
#[require(Name(|| Name::new("Mapping button")), ButtonKind(|| ButtonKind::Normal))]
struct InputButton {
    /// Assigned input.
    input: Option<Input>,
}

/// Stores assigned button with input.
#[derive(Component)]
#[require(Name(|| Name::new("Delete button")), ButtonKind(|| ButtonKind::Symbol))]
struct DeleteButton {
    /// Entity with [`InputButton`].
    button_entity: Entity,
}

#[derive(Component)]
#[require(Name(|| Name::new("Binding dialog")), Dialog)]
struct BindingDialog {
    /// Entity with [`InputButton`].
    button_entity: Entity,
}

#[derive(Component)]
#[require(Name(|| Name::new("Conflict dialog")), Dialog)]
struct ConflictDialog {
    /// Entity with [`InputButton`].
    button_entity: Entity,
    /// Entity with [`InputButton`] that conflicts with [`Self::button_entity`].
    conflict_entity: Entity,
}
