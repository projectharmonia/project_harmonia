mod walls_node;

use bevy::prelude::*;
use project_harmonia_base::{
    asset::info::object_info::{ObjectCategory, ObjectInfo},
    game_world::family::{BuildingMode, FamilyMode},
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, TabContent, TextButtonBundle, Toggled},
    theme::Theme,
};
use strum::IntoEnumIterator;

use crate::hud::objects_node;

pub(super) struct BuildingHudPlugin;

impl Plugin for BuildingHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::set_building_mode.run_if(in_state(FamilyMode::Building)),
        );
    }
}

impl BuildingHudPlugin {
    fn set_building_mode(
        mut building_mode: ResMut<NextState<BuildingMode>>,
        buttons: Query<(Ref<Toggled>, &BuildingMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                info!("changing building mode to `{mode:?}`");
                building_mode.set(mode);
            }
        }
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    theme: &Theme,
    objects_info: &Assets<ObjectInfo>,
) {
    let tabs_entity = parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::FlexEnd,
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .id();

    for mode in BuildingMode::iter() {
        let content_entity = parent
            .spawn(NodeBundle {
                style: Style {
                    align_self: AlignSelf::FlexEnd,
                    padding: theme.padding.normal,
                    column_gap: theme.gap.normal,
                    ..Default::default()
                },
                background_color: theme.panel_color.into(),
                ..Default::default()
            })
            .with_children(|parent| match mode {
                BuildingMode::Objects => {
                    objects_node::setup(
                        parent,
                        tab_commands,
                        theme,
                        objects_info,
                        ObjectCategory::FAMILY_CATEGORIES,
                    );
                }
                BuildingMode::Walls => walls_node::setup(parent, theme),
            })
            .id();

        tab_commands
            .spawn((
                mode,
                TabContent(content_entity),
                ExclusiveButton,
                Toggled(mode == Default::default()),
                TextButtonBundle::symbol(theme, mode.glyph()),
            ))
            .set_parent(tabs_entity);
    }
}
