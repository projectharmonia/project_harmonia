use bevy::prelude::*;
use strum::IntoEnumIterator;

use super::FamilyHud;
use crate::{
    core::{
        asset_metadata::{ObjectCategory, ObjectMetadata},
        family::{BuildingMode, FamilyMode},
        game_state::GameState,
    },
    ui::{
        hud::objects_node,
        theme::Theme,
        widget::button::{ExclusiveButton, TabContent, TextButtonBundle, Toggled},
    },
};

pub(super) struct BuildingHudPlugin;

impl Plugin for BuildingHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::setup_system
                .run_if(in_state(GameState::Family))
                .in_schedule(OnEnter(FamilyMode::Building)),
        )
        .add_system(
            Self::mode_button_system
                .in_set(OnUpdate(GameState::Family))
                .in_set(OnUpdate(FamilyMode::Building)),
        );
    }
}

impl BuildingHudPlugin {
    fn setup_system(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        huds: Query<Entity, With<FamilyHud>>,
    ) {
        commands.entity(huds.single()).with_children(|parent| {
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
                            gap: theme.gap.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .with_children(|parent| match mode {
                        BuildingMode::Objects => {
                            objects_node::setup_objects_node(
                                parent,
                                &mut tab_commands,
                                &theme,
                                &object_metadata,
                                ObjectCategory::FAMILY_CATEGORIES,
                            );
                        }
                        BuildingMode::Walls => setup_walls_node(parent, &theme),
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
        mut building_mode: ResMut<NextState<BuildingMode>>,
        buttons: Query<(Ref<Toggled>, &BuildingMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                building_mode.set(mode);
            }
        }
    }
}

fn setup_walls_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            // Just a stab for instruments.
            parent.spawn((
                ExclusiveButton,
                Toggled(true),
                TextButtonBundle::symbol(theme, "➕"),
            ));
        });
}
