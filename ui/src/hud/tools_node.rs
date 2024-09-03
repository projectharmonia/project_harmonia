use bevy::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use project_harmonia_base::game_world::{
    commands_history::CommandsHistory, family::FamilyMode, WorldState,
};
use project_harmonia_widgets::{button::TextButtonBundle, click::Click, theme::Theme};

pub(super) struct ToolsNodePlugin;

impl Plugin for ToolsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::apply_history_action
                .run_if(in_state(FamilyMode::Building).or_else(in_state(WorldState::City))),
        );
    }
}

impl ToolsNodePlugin {
    fn apply_history_action(
        mut history: CommandsHistory,
        mut click_events: EventReader<Click>,
        buttons: Query<&HistoryButton>,
    ) {
        for button in buttons.iter_many(click_events.read().map(|event| event.0)) {
            match button {
                HistoryButton::Undo => history.undo(),
                HistoryButton::Redo => history.redo(),
            }
        }
    }
}

pub(super) fn setup(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for button in HistoryButton::iter() {
                parent.spawn((button, TextButtonBundle::symbol(theme, button.glyph())));
            }
        });
}

#[derive(Component, EnumIter, Clone, Copy)]
enum HistoryButton {
    Undo,
    Redo,
}

impl HistoryButton {
    fn glyph(&self) -> &'static str {
        match self {
            HistoryButton::Undo => "↩",
            HistoryButton::Redo => "↪",
        }
    }
}
