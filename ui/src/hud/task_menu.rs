use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use project_harmonia_base::game_world::{
    actor::task::{AvailableTasks, TaskSelect},
    family::FamilyMode,
};
use project_harmonia_widgets::{button::ButtonKind, label::LabelKind, theme::Theme};

pub(super) struct TaskMenuPlugin;

impl Plugin for TaskMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<TaskMenu>()
            .add_observer(Self::close.never_param_warn())
            .add_observer(Self::open.never_param_warn());
    }
}

impl TaskMenuPlugin {
    // TODO 0.16: listen for `AvailableTasks` insertion when hierarchy will be available.
    fn open(
        trigger: Trigger<OnAdd, Parent>,
        mut commands: Commands,
        theme: Res<Theme>,
        menu_entity: Option<Single<Entity, With<TaskMenu>>>,
        window: Single<&Window>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
        available_tasks: Query<(&Parent, Option<&Children>), With<AvailableTasks>>,
        names: Query<&Name>,
        tasks: Query<(Entity, &Name)>,
    ) {
        let Ok((parent, children)) = available_tasks.get(trigger.entity()) else {
            return;
        };

        if let Some(menu_entity) = menu_entity {
            info!("closing previous task menu");
            commands.entity(*menu_entity).despawn_recursive();
        }

        let Some(children) = children else {
            debug!("no available tasks found");
            return;
        };

        let name = names.get(**parent).map(|name| &**name).unwrap_or_default();

        info!("showing task menu");
        let cursor_pos = window.cursor_position().unwrap_or_default();
        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn_empty()
                .with_children(|parent| {
                    parent.spawn((LabelKind::Normal, Text::new(name)));

                    for (task_entity, task_name) in tasks.iter_many(children) {
                        parent
                            .spawn(TaskButton { task_entity })
                            .with_child(Text::new(task_name))
                            .observe(Self::request_task);
                    }
                })
                .insert((
                    TaskMenu,
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(cursor_pos.x),
                        top: Val::Px(cursor_pos.y),
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: theme.padding.normal,
                        row_gap: theme.gap.normal,
                        ..Default::default()
                    },
                    theme.panel_background,
                ));
        });
    }

    fn request_task(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        buttons: Query<&TaskButton>,
        menu_entity: Single<Entity, With<TaskMenu>>,
    ) {
        let button = buttons.get(trigger.entity()).unwrap();
        info!("selecting task `{}`", trigger.entity());

        commands.trigger_targets(TaskSelect, button.task_entity);
        commands.entity(*menu_entity).despawn_recursive();
    }

    fn close(
        _trigger: Trigger<Completed<CloseTaskMenu>>,
        mut commands: Commands,
        menu_entity: Single<Entity, With<TaskMenu>>,
    ) {
        info!("closing task menu");
        commands.entity(*menu_entity).despawn_recursive();
    }
}

#[derive(Component)]
#[require(StateScoped::<FamilyMode>(|| StateScoped(FamilyMode::Life)))]
struct TaskMenu;

impl InputContext for TaskMenu {
    const PRIORITY: isize = 1;

    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        ctx.bind::<CloseTaskMenu>()
            .to((KeyCode::Escape, GamepadButton::East));
        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct CloseTaskMenu;

#[derive(Component)]
#[require(Name(|| Name::new("Task button")), ButtonKind(|| ButtonKind::Normal))]
struct TaskButton {
    task_entity: Entity,
}
