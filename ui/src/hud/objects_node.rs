use bevy::prelude::*;

use crate::preview::Preview;
use project_harmonia_base::{
    asset::info::object_info::{ObjectCategory, ObjectInfo},
    game_world::{
        city::{ActiveCity, CityMode},
        family::FamilyMode,
        object::placing_object::PlacingObject,
    },
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, ImageButtonBundle, TabContent, TextButtonBundle, Toggled},
    popup::PopupBundle,
    theme::Theme,
};

pub(super) struct ObjectsNodePlugin;

impl Plugin for ObjectsNodePlugin {
    fn build(&self, app: &mut App) {
        app.observe(Self::untoggle).add_systems(
            Update,
            (Self::start_placing, Self::show_popup, Self::reload_buttons)
                .run_if(in_state(CityMode::Objects).or_else(in_state(FamilyMode::Building))),
        );
    }
}

impl ObjectsNodePlugin {
    fn start_placing(
        mut commands: Commands,
        active_cities: Query<Entity, With<ActiveCity>>,
        buttons: Query<(Entity, &Toggled, &Preview), (Changed<Toggled>, With<ObjectButton>)>,
    ) {
        for (button_entity, toggled, &preview) in &buttons {
            let Preview::Object(id) = preview else {
                panic!("buttons should contain only object previews");
            };

            if toggled.0 {
                debug!("starting spawning object `{id:?}`");
                let placing_entity = commands
                    .spawn(PlacingObject::Spawning(id))
                    .set_parent(active_cities.single())
                    .id();

                commands
                    .entity(button_entity)
                    .insert(ButtonPlacingObject(placing_entity));
            }
        }
    }

    fn show_popup(
        mut commands: Commands,
        theme: Res<Theme>,
        objects_info: Res<Assets<ObjectInfo>>,
        buttons: Query<
            (Entity, &Interaction, &Style, &GlobalTransform, &Preview),
            (Changed<Interaction>, With<ObjectButton>),
        >,
        windows: Query<&Window>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        for (entity, &interaction, style, transform, &preview) in &buttons {
            let Preview::Object(id) = preview else {
                continue;
            };
            if interaction != Interaction::Hovered {
                continue;
            }

            let info = objects_info.get(id).unwrap();
            commands.entity(roots.single()).with_children(|parent| {
                parent
                    .spawn(PopupBundle::new(
                        &theme,
                        windows.single(),
                        entity,
                        style,
                        transform,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_sections([
                            TextSection::new(
                                info.general.name.clone() + "\n\n",
                                theme.label.normal.clone(),
                            ),
                            TextSection::new(
                                format!("{}\n{}", info.general.license, info.general.author,),
                                theme.label.small.clone(),
                            ),
                        ]));
                    });
            });
        }
    }

    fn reload_buttons(
        mut commands: Commands,
        mut change_events: EventReader<AssetEvent<ObjectInfo>>,
        objects_info: Res<Assets<ObjectInfo>>,
        theme: Res<Theme>,
        buttons: Query<(Entity, &Preview), With<ObjectButton>>,
        categories: Query<(&ObjectCategory, &TabContent)>,
    ) {
        for &event in change_events.read() {
            if let AssetEvent::Modified { id } = event {
                debug!("recreating button for asset {id}");

                for (entity, &preview) in &buttons {
                    if let Preview::Object(info_id) = preview {
                        if id == info_id {
                            commands.entity(entity).despawn_recursive();
                            break;
                        }
                    }
                }

                let object_info = objects_info
                    .get(id)
                    .expect("info should always come from file");

                let tab_content = categories.iter().find_map(|(&category, &tab_content)| {
                    if category == object_info.category {
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

    fn untoggle(
        trigger: Trigger<OnRemove, PlacingObject>,
        mut commands: Commands,
        mut buttons: Query<(Entity, &mut Toggled, &ButtonPlacingObject)>,
    ) {
        if let Some((button_entity, mut toggled, _)) = buttons
            .iter_mut()
            .find(|(.., placing_entity)| placing_entity.0 == trigger.entity())
        {
            debug!(
                "untoggling button `{button_entity}` for placing object `{}`",
                trigger.entity()
            );

            toggled.0 = false;
            commands
                .entity(button_entity)
                .remove::<ButtonPlacingObject>();
        }
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    theme: &Theme,
    objects_info: &Assets<ObjectInfo>,
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
                for (id, _) in objects_info
                    .iter()
                    .filter(|(_, info)| info.category == category)
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

#[derive(Bundle)]
struct ObjectButtonBundle {
    object_button: ObjectButton,
    preview: Preview,
    toggled: Toggled,
    image_button_bundle: ImageButtonBundle,
}

impl ObjectButtonBundle {
    fn new(id: AssetId<ObjectInfo>, theme: &Theme) -> Self {
        Self {
            object_button: ObjectButton,
            preview: Preview::Object(id),
            toggled: Toggled(false),
            image_button_bundle: ImageButtonBundle::placeholder(theme),
        }
    }
}

#[derive(Component)]
struct ButtonPlacingObject(Entity);
