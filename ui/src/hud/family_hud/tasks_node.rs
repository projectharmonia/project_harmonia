use bevy::prelude::*;

use project_harmonia_base::game_world::actor::{
    task::{ActiveTask, Task, TaskCancel},
    SelectedActor,
};
use project_harmonia_widgets::{button::ButtonKind, theme::Theme};

pub(super) struct TasksNodePlugin;

impl Plugin for TasksNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::change_actor.never_param_warn())
            .add_observer(Self::add_task.never_param_warn())
            .add_observer(Self::activate_task.never_param_warn())
            .add_observer(Self::cleanup);
    }
}

impl TasksNodePlugin {
    // TODO 0.16: listen for `Task` insertion when hierarchy will be available.
    fn add_task(
        trigger: Trigger<OnAdd, Parent>,
        mut commands: Commands,
        queued_node_entity: Single<Entity, With<QueuedTasksNode>>,
        actor_entity: Single<Entity, With<SelectedActor>>,
        tasks: Query<&Parent, With<Task>>,
    ) {
        let Ok(parent) = tasks.get(trigger.entity()) else {
            return;
        };
        if **parent != *actor_entity {
            return;
        }

        debug!("creating queued task button for `{}`", trigger.entity());

        commands
            .entity(*queued_node_entity)
            .with_children(|parent| {
                spawn_button(parent, trigger.entity());
            });
    }

    fn activate_task(
        trigger: Trigger<OnAdd, ActiveTask>,
        mut commands: Commands,
        active_node_entity: Single<Entity, With<ActiveTasksNode>>,
        buttons: Query<(Entity, &TaskButton)>,
    ) {
        if let Some((button_entity, _)) = buttons
            .iter()
            .find(|(_, task_button)| task_button.task_entity == trigger.entity())
        {
            debug!(
                "turning queued button for `{}` into active",
                trigger.entity()
            );
            commands
                .entity(button_entity)
                .set_parent(*active_node_entity);
        }
    }

    fn change_actor(
        _trigger: Trigger<OnAdd, SelectedActor>,
        mut commands: Commands,
        actor_children: Single<&Children, With<SelectedActor>>,
        queued_node_entity: Single<Entity, With<QueuedTasksNode>>,
        active_node_entity: Single<Entity, With<ActiveTasksNode>>,
        tasks: Query<(Entity, Has<ActiveTask>), With<Task>>,
    ) {
        debug!("reloading actor task buttons");

        commands.entity(*queued_node_entity).despawn_descendants();
        commands.entity(*active_node_entity).despawn_descendants();

        for (task_entity, active) in tasks.iter_many(*actor_children) {
            let node_entity = if active {
                *active_node_entity
            } else {
                *queued_node_entity
            };

            commands.entity(node_entity).with_children(|parent| {
                spawn_button(parent, task_entity);
            });
        }
    }

    fn cancel(
        trigger: Trigger<Pointer<Click>>,
        mut cancel_events: EventWriter<TaskCancel>,
        buttons: Query<&TaskButton>,
    ) {
        let task_button = buttons.get(trigger.entity()).unwrap();
        cancel_events.send(TaskCancel(task_button.task_entity));
    }

    fn cleanup(
        trigger: Trigger<OnRemove, Task>,
        mut commands: Commands,
        buttons: Query<(Entity, &TaskButton)>,
    ) {
        if let Some((entity, _)) = buttons
            .iter()
            .find(|(_, task_button)| task_button.task_entity == trigger.entity())
        {
            debug!("removing task button `{entity}` for `{}`", trigger.entity());
            commands.entity(entity).despawn_recursive();
        }
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    theme: &Theme,
    actor_children: &Children,
    tasks: &Query<(Entity, Has<ActiveTask>), With<Task>>,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    QueuedTasksNode,
                    Node {
                        flex_direction: FlexDirection::ColumnReverse,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        row_gap: theme.gap.normal,
                        padding: theme.padding.normal,
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    for (task_entity, active) in tasks.iter_many(actor_children) {
                        if !active {
                            spawn_button(parent, task_entity);
                        }
                    }
                });

            const MAX_TASKS: usize = 4;
            // Image button is a square
            let Val::Px(width) = theme.button.image.width else {
                panic!("button width should be set in pixels");
            };
            let height = width * MAX_TASKS as f32;

            let UiRect {
                left: Val::Px(left),
                right: Val::Px(right),
                top: Val::Px(top),
                bottom: Val::Px(bottom),
            } = theme.padding.normal
            else {
                panic!("padding should be set in pixels");
            };

            let min_width = Val::Px(width + left + right);
            let min_height = Val::Px(height + top + bottom);

            parent
                .spawn((
                    ActiveTasksNode,
                    Node {
                        min_width,
                        min_height,
                        flex_direction: FlexDirection::Column,
                        row_gap: theme.gap.normal,
                        padding: theme.padding.normal,
                        ..Default::default()
                    },
                    theme.panel_background,
                ))
                .with_children(|parent| {
                    for (task_entity, active) in tasks.iter_many(actor_children) {
                        if active {
                            spawn_button(parent, task_entity);
                        }
                    }
                });
        });
}

fn spawn_button(parent: &mut ChildBuilder, task_entity: Entity) {
    parent
        .spawn(TaskButton { task_entity })
        .with_child(ImageNode::default())
        .observe(TasksNodePlugin::cancel);
}

#[derive(Component)]
#[require(Name(|| Name::new("Active tasks node")), Node)]
struct ActiveTasksNode;

#[derive(Component)]
#[require(Name(|| Name::new("Queued tasks node")), Node)]
struct QueuedTasksNode;

#[derive(Component, Debug)]
#[require(ButtonKind(|| ButtonKind::Image))]
struct TaskButton {
    task_entity: Entity,
}
