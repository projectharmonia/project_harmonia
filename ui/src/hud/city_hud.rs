mod roads_node;

use bevy::prelude::*;
use project_harmonia_base::{
    asset::manifest::{
        object_manifest::{ObjectCategory, ObjectManifest},
        road_manifest::RoadManifest,
    },
    game_world::{city::CityMode, WorldState},
};
use project_harmonia_widgets::{
    button::{ButtonKind, TabContent, Toggled},
    theme::Theme,
};
use strum::IntoEnumIterator;

use crate::hud::{objects_node, tools_node};
use roads_node::RoadsNodePlugin;

pub(super) struct CityHudPlugin;

impl Plugin for CityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RoadsNodePlugin)
            .add_systems(OnEnter(WorldState::City), setup)
            .add_systems(Update, set_city_mode.run_if(in_state(WorldState::City)));
    }
}

fn setup(
    mut commands: Commands,
    mut tab_commands: Commands,
    theme: Res<Theme>,
    asset_server: Res<AssetServer>,
    object_manifests: Res<Assets<ObjectManifest>>,
    road_manifests: Res<Assets<RoadManifest>>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
) {
    debug!("showing city HUD");
    commands.entity(*root_entity).with_children(|parent| {
        parent
            .spawn((
                StateScoped(WorldState::City),
                PickingBehavior::IGNORE,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                tools_node::setup(parent, &theme);

                let tabs_entity = parent
                    .spawn((
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_self: AlignSelf::FlexEnd,
                            padding: theme.padding.normal,
                            ..Default::default()
                        },
                        theme.panel_background,
                    ))
                    .id();

                for mode in CityMode::iter() {
                    let content_entity = parent
                        .spawn((
                            Node {
                                align_self: AlignSelf::FlexEnd,
                                padding: theme.padding.normal,
                                column_gap: theme.gap.normal,
                                ..Default::default()
                            },
                            theme.panel_background,
                        ))
                        .with_children(|parent| match mode {
                            CityMode::Objects => {
                                objects_node::setup(
                                    parent,
                                    &mut tab_commands,
                                    &theme,
                                    &object_manifests,
                                    ObjectCategory::CITY_CATEGORIES,
                                );
                            }
                            CityMode::Roads => roads_node::setup(
                                parent,
                                &mut tab_commands,
                                &asset_server,
                                &theme,
                                &road_manifests,
                            ),
                        })
                        .id();

                    tab_commands
                        .spawn((
                            mode,
                            ButtonKind::Symbol,
                            TabContent(content_entity),
                            Toggled(mode == Default::default()),
                        ))
                        .with_child(Text::new(mode.glyph()))
                        .set_parent(tabs_entity);
                }
            });
    });
}

fn set_city_mode(
    mut commands: Commands,
    buttons: Query<(Ref<Toggled>, &CityMode), Changed<Toggled>>,
) {
    for (toggled, &mode) in &buttons {
        if toggled.0 && !toggled.is_added() {
            info!("changing city mode to `{mode:?}`");
            commands.set_state(mode);
        }
    }
}
