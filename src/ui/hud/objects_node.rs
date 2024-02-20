use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    core::{
        asset::metadata::object_metadata::{ObjectCategory, ObjectMetadata},
        city::ActiveCity,
        family::FamilyMode,
        game_state::GameState,
        object::placing_object::PlacingObject,
    },
    ui::{
        preview::Preview,
        theme::Theme,
        widget::{
            button::{ExclusiveButton, ImageButtonBundle, TabContent, TextButtonBundle, Toggled},
            ui_root::UiRoot,
        },
    },
};

pub(super) struct ObjectsNodePlugin;

impl Plugin for ObjectsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::button_system,
                Self::popup_system,
                Self::toggle_system,
                Self::reload_system,
            )
                .run_if(
                    in_state(GameState::City).or_else(
                        in_state(GameState::Family).and_then(in_state(FamilyMode::Building)),
                    ),
                ),
        );
    }
}

impl ObjectsNodePlugin {
    fn button_system(
        mut commands: Commands,
        active_cities: Query<Entity, With<ActiveCity>>,
        buttons: Query<(&Toggled, &Preview), (Changed<Toggled>, With<ObjectButton>)>,
    ) {
        for (toggled, &preview) in &buttons {
            let Preview::Object(id) = preview else {
                continue;
            };

            if toggled.0 {
                commands
                    .entity(active_cities.single())
                    .with_children(|parent| {
                        parent.spawn(PlacingObject::spawning(id));
                    });
            }
        }
    }

    fn popup_system(
        mut commands: Commands,
        theme: Res<Theme>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        buttons: Query<
            (&Interaction, &Style, &GlobalTransform, &Preview),
            (Changed<Interaction>, With<ObjectButton>),
        >,
        windows: Query<&Window, With<PrimaryWindow>>,
        popups: Query<Entity, With<ObjectPopup>>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        for (&interaction, style, transform, &preview) in &buttons {
            let Preview::Object(id) = preview else {
                continue;
            };

            match interaction {
                Interaction::Hovered => {
                    let (Val::Px(button_width), Val::Px(button_height)) =
                        (style.width, style.height)
                    else {
                        panic!("button size should be set in pixels");
                    };
                    let button_translation = transform.translation();
                    let window = windows.single();
                    let left = button_translation.x - button_width / 2.0;
                    let bottom =
                        window.resolution.height() - button_translation.y + button_height / 2.0;
                    let metadata = object_metadata.get(id).unwrap();

                    commands.entity(roots.single()).with_children(|parent| {
                        parent
                            .spawn((
                                ObjectPopup,
                                NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Column,
                                        padding: theme.padding.normal,
                                        left: Val::Px(left),
                                        bottom: Val::Px(bottom),
                                        position_type: PositionType::Absolute,
                                        ..Default::default()
                                    },
                                    background_color: theme.popup_color.into(),
                                    ..Default::default()
                                },
                            ))
                            .with_children(|parent| {
                                parent.spawn(TextBundle::from_sections([
                                    TextSection::new(
                                        metadata.general.name.clone() + "\n\n",
                                        theme.label.normal.clone(),
                                    ),
                                    TextSection::new(
                                        format!(
                                            "{}\n{}",
                                            metadata.general.license, metadata.general.author,
                                        ),
                                        theme.label.small.clone(),
                                    ),
                                ]));
                            });
                    });
                }
                Interaction::Pressed | Interaction::None => {
                    if let Ok(entity) = popups.get_single() {
                        commands.entity(entity).despawn_recursive();
                    }
                }
            }
        }
    }

    fn toggle_system(
        mut removed_objects: RemovedComponents<PlacingObject>,
        placing_objects: Query<(), With<PlacingObject>>,
        mut buttons: Query<&mut Toggled, With<ObjectButton>>,
    ) {
        if removed_objects.read().count() != 0 {
            // If there is no button, then the object was moved.
            if let Some(mut toggled) = buttons.iter_mut().find(|toggled| toggled.0) {
                if placing_objects.is_empty() {
                    toggled.0 = false;
                }
            }
        }
    }

    fn reload_system(
        mut commands: Commands,
        mut change_events: EventReader<AssetEvent<ObjectMetadata>>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        theme: Res<Theme>,
        buttons: Query<(Entity, &Preview), With<ObjectButton>>,
        categories: Query<(&ObjectCategory, &TabContent)>,
    ) {
        for &event in change_events.read() {
            if let AssetEvent::Modified { id } = event {
                debug!("recreating button for asset {id}");

                for (entity, &preview) in &buttons {
                    if let Preview::Object(metadata_id) = preview {
                        if id == metadata_id {
                            commands.entity(entity).despawn_recursive();
                            break;
                        }
                    }
                }

                let object_metadata = object_metadata
                    .get(id)
                    .expect("metadata should always come from file");

                let tab_content = categories.iter().find_map(|(&category, &tab_content)| {
                    if category == object_metadata.category {
                        Some(tab_content)
                    } else {
                        None
                    }
                });

                if let Some(tab_content) = tab_content {
                    commands.entity(tab_content.0).with_children(|parent| {
                        parent.spawn(ObjectButtonBundle::new(id, &theme));
                    });
                }
            }
        }
    }
}

pub(super) fn setup_objects_node(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    theme: &Theme,
    object_metadata: &Assets<ObjectMetadata>,
    categories: &[ObjectCategory],
) {
    let tabs_entity = parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .id();

    for (index, &category) in categories.iter().enumerate() {
        let content_entity = parent
            .spawn(NodeBundle {
                style: Style {
                    display: Display::Grid,
                    column_gap: theme.gap.normal,
                    row_gap: theme.gap.normal,
                    padding: theme.padding.normal,
                    grid_template_columns: vec![GridTrack::auto(); 8],
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                for (id, _) in object_metadata
                    .iter()
                    .filter(|(_, metadata)| metadata.category == category)
                {
                    parent.spawn(ObjectButtonBundle::new(id, theme));
                }
            })
            .id();

        tab_commands
            .spawn((
                category,
                TabContent(content_entity),
                ExclusiveButton,
                Toggled(index == 0),
                TextButtonBundle::symbol(theme, category.glyph()),
            ))
            .set_parent(tabs_entity);
    }
}

#[derive(Component)]
struct ObjectButton;

#[derive(Component)]
struct ObjectPopup;

#[derive(Bundle)]
struct ObjectButtonBundle {
    object_button: ObjectButton,
    preview: Preview,
    toggled: Toggled,
    exclusive_button: ExclusiveButton,
    image_button_bundle: ImageButtonBundle,
}

impl ObjectButtonBundle {
    fn new(id: AssetId<ObjectMetadata>, theme: &Theme) -> Self {
        Self {
            object_button: ObjectButton,
            preview: Preview::Object(id),
            toggled: Toggled(false),
            exclusive_button: ExclusiveButton,
            image_button_bundle: ImageButtonBundle::placeholder(theme),
        }
    }
}
