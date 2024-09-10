use bevy::prelude::*;
use project_harmonia_base::game_world::family::building::{wall::WallTool, BuildingMode};
use project_harmonia_widgets::{
    button::{ExclusiveButton, TextButtonBundle, Toggled},
    theme::Theme,
};
use strum::IntoEnumIterator;

pub(super) struct WallsNodePlugin;

impl Plugin for WallsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::set_wall_tool.run_if(in_state(BuildingMode::Walls)),
        )
        .add_systems(OnEnter(BuildingMode::Walls), Self::update_wall_tool);
    }
}

impl WallsNodePlugin {
    fn set_wall_tool(
        mut wall_tool: ResMut<NextState<WallTool>>,
        buttons: Query<(Ref<Toggled>, &WallTool), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                info!("changing wall tool to `{mode:?}`");
                wall_tool.set(mode);
            }
        }
    }

    /// Sets tool to the last selected.
    ///
    /// Needed because on swithicng tab the tool resets, but selected button doesn't.
    fn update_wall_tool(
        mut wall_tool: ResMut<NextState<WallTool>>,
        buttons: Query<(&Toggled, &WallTool)>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 {
                debug!("restoring wall tool to `{mode:?}`");
                wall_tool.set(mode);
            }
        }
    }
}

pub(super) fn setup(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            for tool in WallTool::iter() {
                parent.spawn((
                    tool,
                    ExclusiveButton,
                    Toggled(tool == Default::default()),
                    TextButtonBundle::symbol(theme, tool.glyph()),
                ));
            }
        });
}
