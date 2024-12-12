use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use project_harmonia_base::game_world::{
    commands_history::CommandsHistory, family::FamilyMode, WorldState,
};
use project_harmonia_widgets::{button::TextButtonBundle, click::Click, theme::Theme};

pub(super) struct ToolsNodePlugin;

impl Plugin for ToolsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<ToolsNode>()
            .observe(Self::undo)
            .observe(Self::redo)
            .add_systems(
                Update,
                Self::apply_history_action
                    .run_if(in_state(FamilyMode::Building).or_else(in_state(WorldState::City))),
            );
    }
}

impl ToolsNodePlugin {
    fn undo(_trigger: Trigger<Fired<Undo>>, mut history: CommandsHistory) {
        history.undo();
    }

    fn redo(_trigger: Trigger<Fired<Redo>>, mut history: CommandsHistory) {
        history.redo();
    }

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
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    padding: theme.padding.normal,
                    ..Default::default()
                },
                background_color: theme.panel_color.into(),
                ..Default::default()
            },
            ToolsNode,
        ))
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

#[derive(Component)]
struct ToolsNode;

impl InputContext for ToolsNode {
    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();

        ctx.bind::<Redo>()
            .to(KeyCode::KeyZ.with_mod_keys(ModKeys::CONTROL | ModKeys::SHIFT))
            .to(GamepadButtonType::RightThumb)
            .with_conditions(Pulse::new(0.3));
        ctx.bind::<Undo>()
            .to(KeyCode::KeyZ.with_mod_keys(ModKeys::CONTROL))
            .to(GamepadButtonType::LeftThumb)
            .with_conditions(Pulse::new(0.3));

        ctx
    }
}

#[derive(Component, InputAction, Debug)]
#[input_action(output = bool)]
struct Undo;

#[derive(Component, InputAction, Debug)]
#[input_action(output = bool)]
struct Redo;
