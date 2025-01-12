use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use project_harmonia_base::game_world::commands_history::CommandsHistory;
use project_harmonia_widgets::{button::ButtonKind, theme::Theme};

pub(super) struct ToolsNodePlugin;

impl Plugin for ToolsNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<ToolsNode>()
            .add_observer(Self::undo)
            .add_observer(Self::redo);
    }
}

impl ToolsNodePlugin {
    fn undo(_trigger: Trigger<Fired<Undo>>, mut history: CommandsHistory) {
        history.undo();
    }

    fn redo(_trigger: Trigger<Fired<Redo>>, mut history: CommandsHistory) {
        history.redo();
    }

    fn click_undo(_trigger: Trigger<Pointer<Click>>, mut history: CommandsHistory) {
        history.undo();
    }

    fn click_redo(_trigger: Trigger<Pointer<Click>>, mut history: CommandsHistory) {
        history.redo();
    }
}

pub(super) fn setup(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn((
            ToolsNode,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                padding: theme.padding.normal,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            parent
                .spawn(ButtonKind::Symbol)
                .with_child(Text::new("↩"))
                .observe(ToolsNodePlugin::click_undo);
            parent
                .spawn(ButtonKind::Symbol)
                .with_child(Text::new("↪"))
                .observe(ToolsNodePlugin::click_redo);
        });
}

#[derive(Component)]
#[require(Name(|| Name::new("Tools node")), Node)]
struct ToolsNode;

impl InputContext for ToolsNode {
    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();

        ctx.bind::<Redo>()
            .to(KeyCode::KeyZ.with_mod_keys(ModKeys::CONTROL | ModKeys::SHIFT))
            .to(GamepadButton::RightTrigger)
            .with_conditions(Pulse::new(0.3));
        ctx.bind::<Undo>()
            .to(KeyCode::KeyZ.with_mod_keys(ModKeys::CONTROL))
            .to(GamepadButton::LeftTrigger)
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
