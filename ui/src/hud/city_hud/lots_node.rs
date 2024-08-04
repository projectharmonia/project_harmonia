use bevy::prelude::*;
use strum::IntoEnumIterator;

use project_harmonia_base::game_world::{building::lot::LotTool, WorldState};
use project_harmonia_widgets::{
    button::{ExclusiveButton, TextButtonBundle, Toggled},
    theme::Theme,
};

pub(super) struct LotsNodePlugin;

impl Plugin for LotsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::set_lot_tool.run_if(in_state(WorldState::City)),
        );
    }
}

impl LotsNodePlugin {
    fn set_lot_tool(
        mut lot_tool: ResMut<NextState<LotTool>>,
        buttons: Query<(Ref<Toggled>, &LotTool), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                info!("changing lot tool to `{mode:?}`");
                lot_tool.set(mode);
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
