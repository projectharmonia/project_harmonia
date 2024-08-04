use bevy::prelude::*;

use project_harmonia_base::game_world::{
    actor::{
        task::{TaskCancel, TaskState},
        SelectedActor,
    },
    family::FamilyMode,
    WorldState,
};
use project_harmonia_widgets::{button::ImageButtonBundle, click::Click, theme::Theme};

pub(super) struct TasksNodePlugin;

impl Plugin for TasksNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::update_nodes,
                Self::cancel.run_if(in_state(FamilyMode::Life)),
            )
                .run_if(in_state(WorldState::Family)),
        )
        .add_systems(PostUpdate, Self::cleanup);
    }
}

impl TasksNodePlugin {
    fn update_nodes(
        mut commands: Commands,
        theme: Res<Theme>,
        actors: Query<(&Children, Ref<SelectedActor>)>,
        tasks: Query<(Entity, Ref<TaskState>)>,
        queued_task_nodes: Query<Entity, With<QueuedTasksNode>>,
        active_task_nodes: Query<Entity, With<ActiveTasksNode>>,
        buttons: Query<(Entity, &ButtonTask)>,
    ) {
        let (children, selected_actor) = actors.single();

        if selected_actor.is_added() {
            commands
                .entity(queued_task_nodes.single())
                .despawn_descendants();
            commands
                .entity(active_task_nodes.single())
                .despawn_descendants();
        }

        for (task_entity, state) in tasks
            .iter_many(children)
            .filter(|(_, state)| state.is_changed() || selected_actor.is_added())
        {
            match *state {
                TaskState::Queued => {
                    debug!("creating queued task button for `{task_entity}`");
                    commands
                        .entity(queued_task_nodes.single())
                        .with_children(|parent| {
                            parent.spawn((
                                ButtonTask(task_entity),
                                ImageButtonBundle::placeholder(&theme),
                            ));
                        });
                }
                TaskState::Active => {
                    if let Some((button_entity, _)) = buttons
                        .iter()
                        .find(|(_, button_task)| button_task.0 == task_entity)
                    {
                        debug!("turning queued button for `{task_entity}` into active");
                        commands
                            .entity(button_entity)
                            .set_parent(active_task_nodes.single());
                    } else {
                        debug!("creating active task button for `{task_entity}`");
                        commands
                            .entity(active_task_nodes.single())
                            .with_children(|parent| {
                                parent.spawn((
                                    ButtonTask(task_entity),
                                    ImageButtonBundle::placeholder(&theme),
                                ));
                            });
                    }
                }
                TaskState::Cancelled => {
                    debug!("marking button for task `{task_entity}` as cancelled")
                }
            };
        }
    }

    fn cancel(
        mut cancel_events: EventWriter<TaskCancel>,
        mut click_events: EventReader<Click>,
        buttons: Query<&ButtonTask>,
    ) {
        for button_task in buttons.iter_many(click_events.read().map(|event| event.0)) {
            cancel_events.send(TaskCancel(button_task.0));
        }
    }

    fn cleanup(
        mut commands: Commands,
        mut removed_tasks: RemovedComponents<TaskState>,
        buttons: Query<(Entity, &ButtonTask)>,
    ) {
        for task_entity in removed_tasks.read() {
            if let Some((button_entity, _)) = buttons
                .iter()
                .find(|(_, button_task)| button_task.0 == task_entity)
            {
                debug!("removing task button for `{task_entity}`");
                commands.entity(button_entity).despawn_recursive();
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
            parent.spawn((
                QueuedTasksNode,
                NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::ColumnReverse,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        row_gap: theme.gap.normal,
                        padding: theme.padding.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));

            const MAX_TASKS: usize = 4;
            // Image button is a square
            let Val::Px(width) = theme.button.image_button.width else {
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

            parent.spawn((
                ActiveTasksNode,
                NodeBundle {
                    style: Style {
                        min_width,
                        min_height,
                        flex_direction: FlexDirection::Column,
                        row_gap: theme.gap.normal,
                        padding: theme.padding.normal,
                        ..Default::default()
                    },
                    background_color: theme.panel_color.into(),
                    ..Default::default()
                },
            ));
        });
}

#[derive(Component)]
struct ActiveTasksNode;

#[derive(Component)]
struct QueuedTasksNode;

#[derive(Component, Debug)]
struct ButtonTask(Entity);
