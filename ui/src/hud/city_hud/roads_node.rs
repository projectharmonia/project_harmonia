use bevy::prelude::*;
use strum::IntoEnumIterator;

use project_harmonia_base::{
    asset::manifest::road_manifest::RoadManifest,
    game_world::city::{
        road::{placing_road::SpawnRoadId, RoadTool},
        CityMode,
    },
};
use project_harmonia_widgets::{
    button::{ButtonKind, ExclusiveButton, TabContent, Toggled},
    label::LabelKind,
    popup::Popup,
    theme::Theme,
};

pub(super) struct RoadsNodePlugin;

impl Plugin for RoadsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(CityMode::Roads), sync_road_tool)
            .add_systems(
                Update,
                (select, show_popup, set_road_tool).run_if(in_state(CityMode::Roads)),
            );
    }
}

fn select(mut commands: Commands, buttons: Query<(&Toggled, &RoadButton), Changed<Toggled>>) {
    for (toggled, road_button) in &buttons {
        if toggled.0 {
            debug!("selecting road `{:?}` for creation", road_button.0);
            commands.insert_resource(SpawnRoadId(road_button.0));
        }
    }
}

fn show_popup(
    mut commands: Commands,
    manifests: Res<Assets<RoadManifest>>,
    root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    buttons: Query<(Entity, &Interaction, &RoadButton), Changed<Interaction>>,
) {
    for (button_entity, &interaction, &road_button) in &buttons {
        if interaction != Interaction::Hovered {
            continue;
        }

        let manifest = manifests.get(*road_button).unwrap();
        info!("showing popup for road '{}'", manifest.general.name);
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

fn set_road_tool(
    mut commands: Commands,
    buttons: Query<(Ref<Toggled>, &RoadTool), Changed<Toggled>>,
) {
    for (toggled, &mode) in &buttons {
        if toggled.0 && !toggled.is_added() {
            info!("changing road tool to `{mode:?}`");
            commands.set_state(mode);
        }
    }
}

/// Sets tool to the last selected.
///
/// Needed because on swithicng tab the tool resets, but selected button doesn't.
fn sync_road_tool(mut commands: Commands, buttons: Query<(&Toggled, &RoadTool)>) {
    for (toggled, &mode) in &buttons {
        if toggled.0 {
            debug!("syncing road tool to `{mode:?}`");
            commands.set_state(mode);
        }
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    asset_server: &AssetServer,
    theme: &Theme,
    manifests: &Assets<RoadManifest>,
) {
    let tabs_entity = parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .id();

    for tool in RoadTool::iter() {
        let mut button_entity = tab_commands.spawn((
            tool,
            ExclusiveButton,
            Toggled(tool == Default::default()),
            ButtonKind::Symbol,
        ));

        button_entity
            .with_child(Text::new(tool.glyph()))
            .set_parent(tabs_entity);

        if tool == RoadTool::Create {
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
                    for (id, manifest) in manifests.iter() {
                        parent.spawn(RoadButton(id)).with_child(ImageNode {
                            image: asset_server.load(manifest.preview.clone()),
                            ..Default::default()
                        });
                    }
                })
                .id();

            button_entity.insert(TabContent(content_entity));
        }
    }
}

#[derive(Component, Clone, Copy, Deref)]
#[require(
    Name(|| Name::new("Road button")),
    ButtonKind(|| ButtonKind::Image),
    ExclusiveButton
)]
struct RoadButton(AssetId<RoadManifest>);
