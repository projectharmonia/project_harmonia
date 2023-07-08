use bevy::prelude::*;
use strum::IntoEnumIterator;

use crate::{
    core::{
        asset_metadata::{ObjectCategory, ObjectMetadata},
        city::CityMode,
        game_state::GameState,
        lot::LotTool,
    },
    ui::{
        hud::objects_node,
        theme::Theme,
        widget::{
            button::{ExclusiveButton, TabContent, TextButtonBundle, Toggled},
            ui_root::UiRoot,
        },
    },
};

pub(super) struct CityHudPlugin;

impl Plugin for CityHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::setup_system.in_schedule(OnEnter(GameState::City)))
            .add_systems(
                (Self::mode_button_system, Self::tool_button_system)
                    .in_set(OnUpdate(GameState::City)),
            );
    }
}

impl CityHudPlugin {
    fn setup_system(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        object_metadata: Res<Assets<ObjectMetadata>>,
    ) {
        commands
            .spawn((
                UiRoot,
                NodeBundle {
                    style: Style {
                        size: Size::all(Val::Percent(100.0)),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
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

                for mode in CityMode::iter() {
                    let content_entity = parent
                        .spawn(NodeBundle {
                            style: Style {
                                align_self: AlignSelf::FlexEnd,
                                padding: theme.padding.normal,
                                gap: theme.gap.normal,
                                ..Default::default()
                            },
                            background_color: theme.panel_color.into(),
                            ..Default::default()
                        })
                        .with_children(|parent| match mode {
                            CityMode::Objects => {
                                objects_node::setup_objects_node(
                                    parent,
                                    &mut tab_commands,
                                    &theme,
                                    &object_metadata,
                                    ObjectCategory::FAMILY_CATEGORIES,
                                );
                            }
                            CityMode::Lots => setup_lots_node(parent, &theme),
                        })
                        .id();

                    tab_commands
                        .spawn((
                            mode,
                            TabContent(content_entity),
                            ExclusiveButton,
                            Toggled(mode == Default::default()),
                            TextButtonBundle::symbol(&theme, mode.glyph()),
                        ))
                        .set_parent(tabs_entity);
                }
            });
    }

    fn mode_button_system(
        mut city_mode: ResMut<NextState<CityMode>>,
        buttons: Query<(Ref<Toggled>, &CityMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                city_mode.set(mode);
            }
        }
    }

    fn tool_button_system(
        mut lot_tool: ResMut<NextState<LotTool>>,
        buttons: Query<(Ref<Toggled>, &LotTool), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                lot_tool.set(mode);
            }
        }
    }
}

fn setup_lots_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for tool in LotTool::iter() {
                parent.spawn((
                    tool,
                    ExclusiveButton,
                    Toggled(tool == Default::default()),
                    TextButtonBundle::symbol(theme, tool.glyph()),
                ));
            }
        });
}
