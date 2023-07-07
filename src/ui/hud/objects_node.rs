use bevy::{asset::HandleId, prelude::*};

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

// TODO 0.11: Use `run_if`.
#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct ObjectsNodeSet;

pub(super) struct ObjectsNodePlugin;

impl Plugin for ObjectsNodePlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            ObjectsNodeSet.run_if(
                in_state(GameState::City)
                    .or_else(in_state(GameState::Family).and_then(in_state(FamilyMode::Building))),
            ),
        )
        .add_systems((Self::button_system, Self::toggle_system).in_set(ObjectsNodeSet));
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
        // TODO 0.11: Use grid layout.
        let content_entity = parent
            .spawn(NodeBundle {
                style: Style {
                    gap: theme.gap.normal,
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
                        Preview::object(id, theme.button.image.size),
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
