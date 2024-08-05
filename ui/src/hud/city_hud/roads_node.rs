use bevy::{prelude::*, window::PrimaryWindow};
use strum::IntoEnumIterator;

use project_harmonia_base::{
    asset::info::road_info::RoadInfo,
    game_world::{
        building::road::{creating_road::CreatingRoadId, RoadTool},
        city::CityMode,
    },
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, ImageButtonBundle, TabContent, TextButtonBundle, Toggled},
    popup::PopupBundle,
    theme::Theme,
};

pub(super) struct RoadsNodePlugin;

impl Plugin for RoadsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (Self::select, Self::show_popup).run_if(in_state(CityMode::Roads)),
        );
    }
}

impl RoadsNodePlugin {
    fn select(mut commands: Commands, buttons: Query<(&Toggled, &RoadButton), Changed<Toggled>>) {
        for (toggled, road_button) in &buttons {
            if toggled.0 {
                debug!("selecting road `{:?}` for creation", road_button.0);
                commands.insert_resource(CreatingRoadId(road_button.0));
            }
        }
    }

    fn show_popup(
        mut commands: Commands,
        theme: Res<Theme>,
        roads_info: Res<Assets<RoadInfo>>,
        buttons: Query<
            (Entity, &RoadButton, &Interaction, &Style, &GlobalTransform),
            Changed<Interaction>,
        >,
        windows: Query<&Window, With<PrimaryWindow>>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        for (entity, road_button, &interaction, style, transform) in &buttons {
            if interaction != Interaction::Hovered {
                continue;
            }

            let metadata = roads_info.get(road_button.0).unwrap();
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
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    asset_server: &AssetServer,
    theme: &Theme,
    roads_info: &Assets<RoadInfo>,
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

    for tool in RoadTool::iter() {
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
                for (id, info) in roads_info.iter() {
                    parent.spawn((
                        RoadButton(id),
                        Toggled(false),
                        ExclusiveButton,
                        ImageButtonBundle::new(theme, asset_server.load(info.preview.clone())),
                    ));
                }
            })
            .id();

        tab_commands
            .spawn((
                tool,
                TabContent(content_entity),
                ExclusiveButton,
                Toggled(tool == Default::default()),
                TextButtonBundle::symbol(theme, tool.glyph()),
            ))
            .set_parent(tabs_entity);
    }
}

#[derive(Component)]
struct RoadButton(AssetId<RoadInfo>);
