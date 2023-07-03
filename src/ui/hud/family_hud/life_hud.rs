use bevy::prelude::*;

use super::FamilyHud;
use crate::{
    core::{
        actor::ActiveActor,
        family::{ActiveFamily, Budget, FamilyActors, FamilyMode},
        game_state::GameState,
        task::{TaskCancel, TaskState},
    },
    ui::{
        theme::Theme,
        widget::{
            button::{ButtonPlugin, ExclusiveButton, ImageButtonBundle, Toggled},
            click::Click,
            LabelBundle,
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
                Self::tasks_node_setup_system,
                Self::tasks_node_system,
                Self::task_button_system,
                // To run despawn commands after image spawns.
                Self::task_cleanup_system.after(ButtonPlugin::image_init_system),
                Self::budget_system,
                Self::actor_buttons_system,
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
        huds: Query<Entity, With<FamilyHud>>,
        families: Query<(&Budget, &FamilyActors), With<ActiveFamily>>,
        actors: Query<Entity, With<ActiveActor>>,
    ) {
        commands.entity(huds.single()).with_children(|parent| {
            setup_tasks_node(parent, &theme);

            let (&budget, family_actors) = families.single();
            setup_portrait_node(parent, &theme, budget);
            setup_members_node(parent, &theme, family_actors, actors.single());
        });
    }

    fn tasks_node_setup_system(
        mut commands: Commands,
        theme: Res<Theme>,
        actors: Query<&Children, Added<ActiveActor>>,
        tasks: Query<(Entity, &TaskState)>,
        queued_task_nodes: Query<Entity, With<QueuedTasksNode>>,
        active_task_nodes: Query<Entity, With<ActiveTasksNode>>,
    ) {
        let Ok(children) = actors.get_single() else {
            return;
        };

        let queued_entity = queued_task_nodes.single();
        let active_entity = active_task_nodes.single();
        commands.entity(queued_entity).despawn_descendants();
        commands.entity(active_entity).despawn_descendants();

        for (task_entity, state) in tasks.iter_many(children) {
            match *state {
                TaskState::Queued => {
                    commands.entity(queued_entity).with_children(|parent| {
                        parent.spawn((
                            ButtonTask(task_entity),
                            ImageButtonBundle::placeholder(&theme),
                        ));
                    });
                }
                TaskState::Active => {
                    commands.entity(active_entity).with_children(|parent| {
                        parent.spawn((
                            ButtonTask(task_entity),
                            ImageButtonBundle::placeholder(&theme),
                        ));
                    });
                }
                TaskState::Cancelled => continue,
            };
        }
    }

    fn tasks_node_system(
        mut commands: Commands,
        theme: Res<Theme>,
        actors: Query<(&Children, Ref<ActiveActor>)>,
        tasks: Query<(Entity, &TaskState), Changed<TaskState>>,
        queued_task_nodes: Query<Entity, With<QueuedTasksNode>>,
        active_task_nodes: Query<Entity, With<ActiveTasksNode>>,
        buttons: Query<(Entity, &ButtonTask)>,
    ) {
        let (children, active_actor) = actors.single();
        if active_actor.is_added() {
            return;
        }

        for (task_entity, state) in tasks.iter_many(children) {
            match *state {
                TaskState::Queued => {
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
                    let (button_entity, _) = buttons
                        .iter()
                        .find(|(_, button_task)| button_task.0 == task_entity)
                        .expect("all tasks should be queued first");

                    commands
                        .entity(button_entity)
                        .set_parent(active_task_nodes.single());
                }
                TaskState::Cancelled => continue,
            };
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
        mut removed_tasks: RemovedComponents<TaskState>,
        buttons: Query<(Entity, &ButtonTask)>,
    ) {
        for task_entity in &mut removed_tasks {
            if let Some((button_entity, _)) = buttons
                .iter()
                .find(|(_, button_task)| button_task.0 == task_entity)
            {
                commands.entity(button_entity).despawn_recursive();
            }
        }
    }

    fn budget_system(
        families: Query<&Budget, (With<ActiveFamily>, Changed<Budget>)>,
        mut labels: Query<&mut Text, With<BudgetLabel>>,
    ) {
        if let Ok(budget) = families.get_single() {
            labels.single_mut().sections[0].value = budget.to_string();
        }
    }

    fn actor_buttons_system(
        mut commands: Commands,
        actor_buttons: Query<(Ref<Toggled>, &PlayActor), Changed<Toggled>>,
    ) {
        for (toggled, play_actor) in &actor_buttons {
            if toggled.0 && !toggled.is_added() {
                commands.entity(play_actor.0).insert(ActiveActor);
            }
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
                        flex_direction: FlexDirection::Column,
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

fn setup_portrait_node(parent: &mut ChildBuilder, theme: &Theme, budget: Budget) {
    parent
        .spawn(NodeBundle {
            style: Style {
                size: Size::new(Val::Px(180.0), Val::Px(30.0)),
                align_self: AlignSelf::FlexEnd,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn((BudgetLabel, LabelBundle::normal(theme, budget.to_string())));
        });
}

fn setup_members_node(
    parent: &mut ChildBuilder,
    theme: &Theme,
    actors: &FamilyActors,
    active_entity: Entity,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                gap: theme.gap.normal,
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for &entity in actors.iter() {
                parent.spawn((
                    PlayActor(entity),
                    ExclusiveButton,
                    Toggled(entity == active_entity),
                    ImageButtonBundle::placeholder(theme),
                ));
            }
        });
}

#[derive(Component)]
struct ActiveTasksNode;

#[derive(Component)]
struct QueuedTasksNode;

#[derive(Component)]
struct ButtonTask(Entity);

#[derive(Component)]
struct BudgetLabel;

#[derive(Component)]
struct PlayActor(Entity);
