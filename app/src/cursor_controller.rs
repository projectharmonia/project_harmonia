use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub(super) struct CursorControllerPlugin;

impl Plugin for CursorControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<CursorController>()
            .add_systems(Startup, Self::setup)
            .add_observer(Self::update_position);
    }
}

impl CursorControllerPlugin {
    fn setup(mut commands: Commands) {
        commands.spawn(CursorController);
    }

    fn update_position(trigger: Trigger<Fired<MoveCursor>>, mut window: Single<&mut Window>) {
        if let Some(cursor_pos) = window.cursor_position() {
            window.set_cursor_position(Some(cursor_pos + trigger.value));
        }
    }
}

#[derive(Component)]
#[require(Name(|| Name::new("Cursor controller")))]
struct CursorController;

impl InputContext for CursorController {
    const PRIORITY: isize = -1;

    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        ctx.bind::<MoveCursor>()
            .to(GamepadStick::Left)
            .with_modifiers((Negate::y(), Scale::splat(8.0)));
        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct MoveCursor;
