use bevy::prelude::*;

use crate::preview::Preview;
use project_harmonia_base::{
    asset::manifest::object_manifest::{ObjectCategory, ObjectManifest},
    game_world::{
        city::{ActiveCity, CityMode},
        family::FamilyMode,
        object::placing_object::PlacingObject,
    },
};
use project_harmonia_widgets::{
    button::{ButtonKind, ExclusiveButton, TabContent, Toggled},
    label::LabelKind,
    popup::Popup,
    theme::Theme,
};

pub(super) struct ObjectsNodePlugin;

impl Plugin for ObjectsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(untoggle).add_systems(
            Update,
            (show_popup, reload_buttons)
                .run_if(in_state(CityMode::Objects).or(in_state(FamilyMode::Building))),
        );
    }
}

fn show_popup(
    mut commands: Commands,
    manifests: Res<Assets<ObjectManifest>>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    buttons: Query<(Entity, &Interaction, &ObjectButton), Changed<Interaction>>,
) {
    for (button_entity, &interaction, &button) in &buttons {
        if interaction != Interaction::Hovered {
            continue;
        }

        let manifest = manifests.get(*button).unwrap();
        info!("showing popup for object '{}'", manifest.general.name);
        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn(Popup { button_entity })
                .with_children(|parent| {
                    parent
                        .spawn((
                            LabelKind::Normal,
                            Text::new(manifest.general.name.clone() + "\n\n"),
                        ))
                        .with_child((
                            LabelKind::Small,
                            TextSpan::new(format!(
                                "{}\n{}",
                                manifest.general.license, manifest.general.author,
                            )),
                        ));
                });
        });
    }
}

fn reload_buttons(
    mut commands: Commands,
    mut change_events: EventReader<AssetEvent<ObjectManifest>>,
    manifests: Res<Assets<ObjectManifest>>,
    buttons: Query<(Entity, &ObjectButton)>,
    categories: Query<(&ObjectCategory, &TabContent)>,
) {
    for &event in change_events.read() {
        let AssetEvent::Modified { id } = event else {
            continue;
        };

        debug!("recreating button for asset {id}");

        // Fully remove the button because category may change.
        for (entity, &button) in &buttons {
            if id == *button {
                commands.entity(entity).despawn_recursive();
                break;
            }
        }

        let manifest = manifests
            .get(id)
            .expect("manifest should always come from file");

        let tab_content = categories.iter().find_map(|(&category, &tab_content)| {
            if category == manifest.category {
                Some(tab_content)
            } else {
                None
            }
        });

        if let Some(tab_content) = tab_content {
            commands.entity(tab_content.0).with_children(|parent| {
                parent
                    .spawn(ObjectButton(id))
                    .with_child(Preview::Object(id))
                    .observe(start_placing);
            });
        }
    }
}

fn untoggle(
    trigger: Trigger<OnRemove, PlacingObjectButton>,
    objects: Query<&PlacingObjectButton>,
    mut buttons: Query<&mut Toggled>,
) {
    let placing_button = *objects.get(trigger.entity()).unwrap();
    if let Ok(mut toggled) = buttons.get_mut(*placing_button) {
        debug!(
            "untoggling button `{}` for placing object `{}`",
            *placing_button,
            trigger.entity()
        );
        **toggled = false
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    theme: &Theme,
    manifests: &Assets<ObjectManifest>,
    categories: &[ObjectCategory],
) {
    let tabs_entity = parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .id();

    for (index, &category) in categories.iter().enumerate() {
        let content_entity = parent
            .spawn(Node {
                display: Display::Grid,
                column_gap: theme.gap.normal,
                row_gap: theme.gap.normal,
                padding: theme.padding.normal,
                grid_template_columns: vec![GridTrack::auto(); 8],
                ..Default::default()
            })
            .with_children(|parent| {
                for (id, _) in manifests
                    .iter()
                    .filter(|(_, manifest)| manifest.category == category)
                {
                    parent
                        .spawn(ObjectButton(id))
                        .with_child(Preview::Object(id))
                        .observe(start_placing);
                }
            })
            .id();

        tab_commands
            .spawn((
                category,
                ButtonKind::Symbol,
                TabContent(content_entity),
                Toggled(index == 0),
            ))
            .with_child(Text::new(category.glyph()))
            .set_parent(tabs_entity);
    }
}

fn start_placing(
    trigger: Trigger<Pointer<Click>>,
    mut commands: Commands,
    city_entity: Single<Entity, With<ActiveCity>>,
    placing_entity: Option<Single<Entity, With<PlacingObject>>>,
    buttons: Query<&ObjectButton>,
) {
    let id = **buttons.get(trigger.entity()).unwrap();

    debug!("starting spawning object `{id:?}`");

    if let Some(placing_entity) = placing_entity {
        commands.entity(*placing_entity).despawn_recursive();
    }

    commands.entity(*city_entity).with_children(|parent| {
        parent.spawn((
            PlacingObject::Spawning(id),
            PlacingObjectButton(trigger.entity()),
        ));
    });
}

#[derive(Component, Clone, Copy, Deref)]
#[require(ButtonKind(|| ButtonKind::Image), ExclusiveButton)]
struct ObjectButton(AssetId<ObjectManifest>);

#[derive(Component, Clone, Copy, Deref)]
struct PlacingObjectButton(Entity);
