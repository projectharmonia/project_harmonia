mod walls_node;

use bevy::prelude::*;
use project_harmonia_base::{
    asset::manifest::object_manifest::{ObjectCategory, ObjectManifest},
    game_world::family::{building::BuildingMode, FamilyMode},
};
use project_harmonia_widgets::{
    button::{ButtonKind, TabContent, Toggled},
    theme::Theme,
};
use strum::IntoEnumIterator;

use crate::hud::{objects_node, tools_node};
use walls_node::WallsNodePlugin;

pub(super) struct BuildingHudPlugin;

impl Plugin for BuildingHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WallsNodePlugin)
            .add_systems(OnEnter(FamilyMode::Building), Self::sync_building_mode);
    }
}

impl BuildingHudPlugin {
    fn set_building_mode(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        buttons: Query<&BuildingMode>,
    ) {
        let mode = *buttons.get(trigger.entity()).unwrap();
        info!("changing building mode to `{mode:?}`");
        commands.set_state(mode);
    }

    /// Sets building mode to the last selected.
    ///
    /// Needed because on swithicng tab the mode resets, but selected button doesn't.
    fn sync_building_mode(mut commands: Commands, buttons: Query<(&Toggled, &BuildingMode)>) {
        for (toggled, &mode) in &buttons {
            if toggled.0 {
                debug!("syncing building mode to `{mode:?}`");
                commands.set_state(mode);
            }
        }
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    theme: &Theme,
    object_manifests: &Assets<ObjectManifest>,
) {
    tools_node::setup(parent, theme);

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

    for mode in BuildingMode::iter() {
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
                BuildingMode::Objects => {
                    objects_node::setup(
                        parent,
                        tab_commands,
                        theme,
                        object_manifests,
                        ObjectCategory::FAMILY_CATEGORIES,
                    );
                }
                BuildingMode::Walls => walls_node::setup(parent),
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
            .set_parent(tabs_entity)
            .observe(BuildingHudPlugin::set_building_mode);
    }
}
