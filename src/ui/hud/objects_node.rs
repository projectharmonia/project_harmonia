use bevy::{asset::HandleId, prelude::*, window::PrimaryWindow};

use crate::{
    core::{
        asset_metadata::{ObjectCategory, ObjectMetadata},
        city::ActiveCity,
        family::FamilyMode,
        game_state::GameState,
        object::placing_object::PlacingObject,
    },
    ui::{
        preview::Preview,
        theme::Theme,
        widget::button::{
            ExclusiveButton, ImageButtonBundle, TabContent, TextButtonBundle, Toggled,
        },
    },
};

pub(super) struct ObjectsNodePlugin;

impl Plugin for ObjectsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (Self::button_system, Self::popup_system, Self::toggle_system).run_if(
                in_state(GameState::City)
                    .or_else(in_state(GameState::Family).and_then(in_state(FamilyMode::Building))),
            ),
        );
    }
}

impl ObjectsNodePlugin {
    fn button_system(
        mut commands: Commands,
        active_cities: Query<Entity, With<ActiveCity>>,
        buttons: Query<(&Toggled, &MetadataId), Changed<Toggled>>,
    ) {
        for (toggled, id) in &buttons {
            if toggled.0 {
                commands
                    .entity(active_cities.single())
                    .with_children(|parent| {
                        parent.spawn(PlacingObject::spawning(id.0));
                    });
            }
        }
    }

    fn popup_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        theme: Res<Theme>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        buttons: Query<(&Interaction, &GlobalTransform, &MetadataId), Changed<Interaction>>,
        windows: Query<&Window, With<PrimaryWindow>>,
        popups: Query<Entity, With<ObjectPopup>>,
    ) {
        for (&interaction, transform, id) in &buttons {
            match interaction {
                Interaction::Pressed => continue,
                Interaction::Hovered => {
                    let (Val::Px(button_width), Val::Px(button_height)) = (
                        theme.button.image_button.width,
                        theme.button.image_button.height,
                    ) else {
                        panic!("button size should be set in pixels");
                    };
                    let button_translation = transform.translation();
                    let window = windows.single();
                    let left = button_translation.x - button_width / 2.0;
                    let bottom =
                        window.resolution.height() - button_translation.y + button_height / 2.0;

                    let metadata_path = asset_server
                        .get_handle_path(id.0)
                        .expect("spawning object metadata should have a path");
                    let object_metadata = object_metadata
                        .get(&asset_server.get_handle(id.0))
                        .unwrap_or_else(|| {
                            panic!("{metadata_path:?} should correspond to metadata")
                        });

                    commands
                        .spawn((
                            ObjectPopup,
                            NodeBundle {
                                style: Style {
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
                            parent.spawn(TextBundle::from_section(
                                &object_metadata.general.name,
                                theme.label.normal.clone(),
                            ));
                        });
                }
                Interaction::None => {
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
        mut buttons: Query<&mut Toggled, With<MetadataId>>,
    ) {
        if removed_objects.iter().count() != 0 {
            // If there is no button, then the object was moved.
            if let Some(mut toggled) = buttons.iter_mut().find(|toggled| toggled.0) {
                if placing_objects.is_empty() {
                    toggled.0 = false;
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
                    parent.spawn((
                        MetadataId(id),
                        Preview::object(id, &theme.button.image),
                        Toggled(false),
                        ExclusiveButton,
                        ImageButtonBundle::placeholder(theme),
                    ));
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
struct MetadataId(HandleId);

#[derive(Component)]
struct ObjectPopup;
