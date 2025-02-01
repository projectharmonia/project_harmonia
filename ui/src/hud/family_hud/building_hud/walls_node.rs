use bevy::prelude::*;
use project_harmonia_base::game_world::family::building::{wall::WallTool, BuildingMode};
use project_harmonia_widgets::button::{ButtonKind, ExclusiveButton, Toggled};
use strum::IntoEnumIterator;

pub(super) struct WallsNodePlugin;

impl Plugin for WallsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BuildingMode::Walls), sync_wall_tool)
            .add_systems(Update, set_wall_tool.run_if(in_state(BuildingMode::Walls)));
    }
}

fn set_wall_tool(
    mut commands: Commands,
    buttons: Query<(Ref<Toggled>, &WallTool), Changed<Toggled>>,
) {
    for (toggled, &mode) in &buttons {
        if toggled.0 && !toggled.is_added() {
            info!("changing wall tool to `{mode:?}`");
            commands.set_state(mode);
        }
    }
}

/// Sets tool to the last selected.
///
/// Needed because on swithicng tab the tool resets, but selected button doesn't.
fn sync_wall_tool(mut commands: Commands, buttons: Query<(&Toggled, &WallTool)>) {
    for (toggled, &mode) in &buttons {
        if toggled.0 {
            debug!("syncing wall tool to `{mode:?}`");
            commands.set_state(mode);
        }
    }
}

pub(super) fn setup(parent: &mut ChildBuilder) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|parent| {
            for tool in WallTool::iter() {
                parent
                    .spawn((
                        tool,
                        ButtonKind::Symbol,
                        ExclusiveButton,
                        Toggled(tool == Default::default()),
                    ))
                    .with_child(Text::new(tool.glyph()));
            }
        });
}
