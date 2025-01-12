use std::fmt::Write;

use bevy::{input::keyboard::KeyboardInput, prelude::*, reflect::GetPath};
use strum::{EnumIter, IntoEnumIterator};

use project_harmonia_base::settings::{Settings, SettingsApply};
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
        app.add_observer(Self::setup).add_systems(
            Update,
            (
                Self::update_mapping_text,
                Self::read_binding.never_param_warn(),
            )
                .run_if(any_with_component::<SettingsMenu>),
        );
    }
}

impl SettingsMenuPlugin {
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
                        align_self: AlignSelf::Center,
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
                            SettingsTab::Video => setup_video_tab(parent, &theme, &settings),
                            SettingsTab::Keyboard => setup_keyboard_tab(parent, &theme, &settings),
                            SettingsTab::Developer => {
                                setup_developer_tab(parent, &theme, &settings)
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
                                .observe(Self::ok);
                            parent
                                .spawn(ButtonKind::Normal)
                                .with_child(Text::new("Cancel"))
                                .observe(Self::cancel);
                        });
                });
        });
    }

    fn update_mapping_text(
        buttons: Query<(&MappingButton, &Children), Changed<MappingButton>>,
        mut text: Query<&mut Text>,
    ) {
        for (mapping, children) in &buttons {
            let mut iter = text.iter_many_mut(children);
            let mut text = iter.fetch_next().unwrap();
            text.clear();
            if let Some(key) = mapping.key {
                write!(text, "{key:?}").unwrap();
            } else {
                write!(text, "Empty").unwrap();
            };
        }
    }

    fn start_binding(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
        buttons: Query<&MappingButton>,
    ) {
        let mapping = buttons.get(trigger.entity()).unwrap();
        info!("starting binding for '{}'", mapping.name);

        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn(BindingDialog::new(trigger.entity()))
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
                            parent.spawn((
                                BindingLabel,
                                Text::new(format!("Binding \"{}\", press any key", mapping.name)),
                            ));
                            parent
                                .spawn(Node {
                                    column_gap: theme.gap.normal,
                                    ..Default::default()
                                })
                                .with_children(|parent| {
                                    parent
                                        .spawn((
                                            ReplaceButton,
                                            Node {
                                                // Replace is hidden by default and will be
                                                // displayed only in case of binding conflict.
                                                display: Display::None,
                                                ..Default::default()
                                            },
                                        ))
                                        .with_child(Text::new("Replace"))
                                        .observe(Self::replace_binding);

                                    parent
                                        .spawn(ButtonKind::Normal)
                                        .with_child(Text::new("Delete"))
                                        .observe(Self::delete_binding);

                                    parent
                                        .spawn(ButtonKind::Normal)
                                        .with_child(Text::new("Cancel"))
                                        .observe(Self::cancel_binding);
                                });
                        });
                });
        });
    }

    fn read_binding(
        mut commands: Commands,
        mut key_events: EventReader<KeyboardInput>,
        dialog: Single<(Entity, &mut BindingDialog)>,
        mut buttons: Query<(Entity, &mut MappingButton)>,
        mut labels: Query<&mut Text, With<BindingLabel>>,
        mut replace_nodes: Query<&mut Node, With<ReplaceButton>>,
    ) {
        let Some(&KeyboardInput { key_code, .. }) = key_events.read().last() else {
            return;
        };

        let (dialog_entity, mut dialog) = dialog.into_inner();
        if let Some((conflict_entity, mapping)) = buttons
            .iter()
            .find(|(_, mapping)| mapping.key == Some(key_code))
        {
            info!("found conflict with '{}' for `{key_code:?}`", mapping.name);
            **labels.single_mut() =
                format!("\"{key_code:?}\" is already used by \"{:?}\"", mapping.name);

            dialog.conflict_button = Some(conflict_entity);

            replace_nodes.single_mut().display = Display::Flex;
        } else {
            let (_, mut mapping) = buttons
                .get_mut(dialog.binding_button)
                .expect("binding dialog should point to a button with mapping");
            info!("assigning `{key_code:?}` to '{}'", mapping.name);
            mapping.key = Some(key_code);
            commands.entity(dialog_entity).despawn_recursive();
        }
    }

    fn replace_binding(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog: Single<(Entity, &BindingDialog)>,
        mut buttons: Query<&mut MappingButton>,
    ) {
        let (dialog_entity, dialog) = *dialog;
        let mut conflict_mapping = dialog
            .conflict_button
            .and_then(|entity| buttons.get_mut(entity).ok())
            .expect("binding conflict should point to a button");
        let input_kind = conflict_mapping.key;
        conflict_mapping.key = None;

        let mut mapping = buttons
            .get_mut(dialog.binding_button)
            .expect("binding should point to a button");
        mapping.key = input_kind;

        info!("reassigning binding to '{}'", mapping.name);
        commands.entity(dialog_entity).despawn_recursive();
    }

    fn delete_binding(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog: Single<(Entity, &BindingDialog)>,
        mut buttons: Query<&mut MappingButton>,
    ) {
        let (entity, dialog) = *dialog;
        let mut mapping = buttons
            .get_mut(dialog.binding_button)
            .expect("binding should point to a button");
        mapping.key = None;

        info!("deleting binding for '{}'", mapping.name);
        commands.entity(entity).despawn_recursive();
    }

    fn cancel_binding(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        dialog_entity: Single<Entity, With<BindingDialog>>,
    ) {
        info!("cancelling binding");
        commands.entity(*dialog_entity).despawn_recursive();
    }

    fn ok(
        _trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        mut settings: ResMut<Settings>,
        menu_entity: Single<Entity, With<SettingsMenu>>,
        buttons: Query<(&MappingButton, &SettingsField)>,
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
        for (mapping, field) in &buttons {
            if let Some(key) = mapping.key {
                let field_value = settings
                    .path_mut::<Vec<KeyCode>>(field.0)
                    .expect("fields with mappings should be stored as Vec");
                field_value.push(key);
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
}

/// Creates [`SettingsField`] from passed field.
macro_rules! settings_field {
    ($path:expr) => {{
        let _validate_field = &$path;
        SettingsField(stringify!($path))
    }};
}

#[derive(Component, Clone, Copy)]
struct SettingsField(&'static str);

fn setup_video_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) -> Entity {
    parent
        .spawn(Node {
            padding: theme.padding.normal,
            row_gap: theme.gap.normal,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|parent| {
            let video = &settings.video;
            parent
                .spawn((
                    Checkbox(video.fullscreen),
                    settings_field!(video.fullscreen),
                ))
                .with_child(Text::new("Fullscreen"));
        })
        .id()
}

const INPUTS_PER_ACTION: usize = 3;

fn setup_keyboard_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) -> Entity {
    parent
        .spawn(Node {
            display: Display::Grid,
            column_gap: theme.gap.normal,
            row_gap: theme.gap.normal,
            grid_template_columns: vec![GridTrack::auto(); INPUTS_PER_ACTION + 1],
            ..Default::default()
        })
        .with_children(|parent| {
            let keyboard = &settings.keyboard;
            setup_action_row(
                parent,
                "Camera forward",
                &keyboard.camera_forward,
                settings_field!(keyboard.camera_forward),
            );
            setup_action_row(
                parent,
                "Camera left",
                &keyboard.camera_left,
                settings_field!(keyboard.camera_left),
            );
            setup_action_row(
                parent,
                "Camera backward",
                &keyboard.camera_backward,
                settings_field!(keyboard.camera_backward),
            );
            setup_action_row(
                parent,
                "Camera right",
                &keyboard.camera_right,
                settings_field!(keyboard.camera_right),
            );
            setup_action_row(
                parent,
                "Rotate left",
                &keyboard.rotate_left,
                settings_field!(keyboard.rotate_left),
            );
            setup_action_row(
                parent,
                "Rotate right",
                &keyboard.rotate_right,
                settings_field!(keyboard.rotate_right),
            );
            setup_action_row(
                parent,
                "Zoom in",
                &keyboard.zoom_in,
                settings_field!(keyboard.zoom_in),
            );
            setup_action_row(
                parent,
                "Zoom out",
                &keyboard.zoom_out,
                settings_field!(keyboard.zoom_out),
            );
            setup_action_row(
                parent,
                "Delete object",
                &keyboard.delete,
                settings_field!(keyboard.delete),
            );
            setup_action_row(
                parent,
                "Free placement",
                &keyboard.free_placement,
                settings_field!(keyboard.free_placement),
            );
            setup_action_row(
                parent,
                "Ordinal placement",
                &keyboard.ordinal_placement,
                settings_field!(keyboard.ordinal_placement),
            );
        })
        .id()
}

fn setup_action_row(
    parent: &mut ChildBuilder,
    name: &'static str,
    keys: &[KeyCode],
    field: SettingsField,
) {
    parent.spawn((LabelKind::Normal, Text::new(name)));
    for index in 0..INPUTS_PER_ACTION {
        parent
            .spawn((
                field,
                MappingButton {
                    name,
                    key: keys.get(index).copied(),
                },
            ))
            .with_child(Text::default())
            .observe(SettingsMenuPlugin::start_binding);
    }
}

fn setup_developer_tab(parent: &mut ChildBuilder, theme: &Theme, settings: &Settings) -> Entity {
    parent
        .spawn(Node {
            padding: theme.padding.normal,
            row_gap: theme.gap.normal,
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|parent| {
            let developer = &settings.developer;
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

#[derive(Component)]
#[require(Name(|| Name::new("Replace button")), ButtonKind(|| ButtonKind::Normal))]
struct ReplaceButton;

/// Stores information about button mapping.
#[derive(Component)]
#[require(Name(|| Name::new("Mapping button")), ButtonKind(|| ButtonKind::Normal))]
struct MappingButton {
    name: &'static str,
    key: Option<KeyCode>,
}

#[derive(Component)]
#[require(Dialog)]
struct BindingDialog {
    binding_button: Entity,
    conflict_button: Option<Entity>,
}

impl BindingDialog {
    fn new(binding_button: Entity) -> Self {
        Self {
            binding_button,
            conflict_button: None,
        }
    }
}

/// Marker for label with binding dialog text.
#[derive(Component)]
#[require(Name(|| Name::new("Binding label")), LabelKind(|| LabelKind::Normal))]
struct BindingLabel;
