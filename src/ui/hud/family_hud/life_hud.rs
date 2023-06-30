use bevy::prelude::*;

use super::FamilyHudRoot;
use crate::{
    core::{
        family::FamilyMode,
        game_state::GameState,
        task::{ActiveTask, QueuedTask, TaskCancel},
    },
    ui::{
        theme::Theme,
        widget::{
            button::{ButtonPlugin, ImageButtonBundle},
            click::Click,
        },
    },
};

pub(super) struct LifeHudPlugin;

impl Plugin for LifeHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::setup_system
                .run_if(in_state(GameState::Family))
                .in_schedule(OnEnter(FamilyMode::Life)),
        )
        .add_systems(
            (
                Self::task_queue_system,
                Self::task_activation_system,
                Self::task_button_system,
                // To run despawn commands after image spawns.
                Self::task_cleanup_system.after(ButtonPlugin::image_init_system),
            )
                .in_set(OnUpdate(GameState::Family))
                .in_set(OnUpdate(FamilyMode::Life)),
        );
    }
}

impl LifeHudPlugin {
    fn setup_system(
        mut commands: Commands,
        theme: Res<Theme>,
        roots: Query<Entity, With<FamilyHudRoot>>,
    ) {
        commands.entity(roots.single()).with_children(|parent| {
            setup_tasks_node(parent, &theme);
        });
    }

    fn task_queue_system(
        mut commands: Commands,
        theme: Res<Theme>,
        tasks: Query<Entity, Added<QueuedTask>>,
        task_nodes: Query<Entity, With<QueuedTasksNode>>,
    ) {
        for entity in &tasks {
            commands
                .entity(task_nodes.single())
                .with_children(|parent| {
                    parent.spawn((ButtonTask(entity), ImageButtonBundle::placeholder(&theme)));
                });
        }
    }

    fn task_activation_system(
        mut commands: Commands,
        theme: Res<Theme>,
        tasks: Query<Entity, Added<ActiveTask>>,
        task_nodes: Query<Entity, With<ActiveTasksNode>>,
    ) {
        for entity in &tasks {
            commands
                .entity(task_nodes.single())
                .with_children(|parent| {
                    parent.spawn((ButtonTask(entity), ImageButtonBundle::placeholder(&theme)));
                });
        }
    }

    fn task_button_system(
        mut cancel_events: EventWriter<TaskCancel>,
        mut click_events: EventReader<Click>,
        buttons: Query<&ButtonTask>,
    ) {
        for event in &mut click_events {
            if let Ok(button_task) = buttons.get(event.0) {
                cancel_events.send(TaskCancel(button_task.0));
            }
        }
    }

    fn task_cleanup_system(
        mut commands: Commands,
        mut unqueued_tasks: RemovedComponents<QueuedTask>,
        mut deactivated_tasks: RemovedComponents<ActiveTask>,
        buttons: Query<(Entity, &ButtonTask)>,
    ) {
        for task_entity in unqueued_tasks.iter().chain(&mut deactivated_tasks) {
            let (button_entity, _) = buttons
                .iter()
                .find(|(_, button_task)| button_task.0 == task_entity)
                .expect("all tasks should have corresponding buttons");
            commands.entity(button_entity).despawn_recursive();
        }
    }
}

fn setup_tasks_node(parent: &mut ChildBuilder, theme: &Theme) {
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
                        size: Size::all(Val::Percent(100.0)),
                        gap: theme.gap.normal,
                        padding: theme.padding.normal,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ));

            const MAX_TASKS: usize = 4;
            // Image button is a square
            let width = theme.button.image_button.size.width;
            let height = width * MAX_TASKS as f32;

            let min_width = width
                .try_add(theme.padding.normal.left)
                .and_then(|val| val.try_add(theme.padding.normal.right))
                .expect("button size and padding should be set pixels");
            let min_height = height
                .try_add(theme.padding.normal.top)
                .and_then(|val| val.try_add(theme.padding.normal.bottom))
                .expect("button size and padding should be set pixels");
            parent.spawn((
                ActiveTasksNode,
                NodeBundle {
                    style: Style {
                        min_size: Size::new(min_width, min_height),
                        gap: theme.gap.normal,
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

#[derive(Component)]
struct ButtonTask(Entity);
