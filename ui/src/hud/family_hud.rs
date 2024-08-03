use bevy::prelude::*;
use strum::{EnumIter, IntoEnumIterator};

use super::objects_node;
use crate::preview::Preview;
use project_harmonia_base::{
    asset::info::object_info::{ObjectCategory, ObjectInfo},
    game_world::{
        actor::{
            needs::{Need, NeedGlyph},
            task::{TaskCancel, TaskState},
            SelectedActor,
        },
        family::{Budget, BuildingMode, FamilyMembers, FamilyMode, FamilyPlugin, SelectedFamily},
        WorldState,
    },
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, ImageButtonBundle, TabContent, TextButtonBundle, Toggled},
    click::Click,
    label::LabelBundle,
    progress_bar::{ProgressBar, ProgressBarBundle},
    theme::Theme,
};

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(WorldState::Family),
            Self::setup.after(FamilyPlugin::select),
        )
        .add_systems(
            Update,
            (
                Self::set_family_mode,
                Self::update_task_nodes,
                Self::update_need_bars,
                Self::update_budget,
                Self::set_building_mode.run_if(in_state(FamilyMode::Building)),
                (Self::cancel_task, Self::select_actor).run_if(in_state(FamilyMode::Life)),
            )
                .run_if(in_state(WorldState::Family)),
        )
        .add_systems(PostUpdate, (Self::cleanup_tasks, Self::cleanup_need_bars));
    }
}

impl FamilyHudPlugin {
    fn setup(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        objects_info: Res<Assets<ObjectInfo>>,
        families: Query<(&Budget, &FamilyMembers), With<SelectedFamily>>,
        actors: Query<Entity, With<SelectedActor>>,
    ) {
        debug!("showing family hud");
        commands
            .spawn((
                StateScoped(WorldState::Family),
                NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .with_children(|parent| {
                let tabs_entity = parent
                    .spawn(NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            right: Val::Px(0.0),
                            padding: theme.padding.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    })
                    .id();

                for mode in FamilyMode::iter() {
                    let content_entity = parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                ..Default::default()
                            },
                            ..Default::default()
                        })
                        .with_children(|parent| match mode {
                            FamilyMode::Life => {
                                setup_tasks_node(parent, &theme);

                                let (&budget, members) = families.single();
                                setup_portrait_node(parent, &theme, budget);
                                setup_members_node(parent, &theme, members, actors.single());
                                setup_info_node(parent, &mut tab_commands, &theme);
                            }
                            FamilyMode::Building => {
                                setup_building_hud(parent, &mut tab_commands, &theme, &objects_info)
                            }
                        })
                        .id();

                    tab_commands
                        .spawn((
                            mode,
                            TabContent(content_entity),
                            ExclusiveButton,
                            Toggled(mode == Default::default()),
                            TextButtonBundle::symbol(&theme, mode.glyph()),
                        ))
                        .set_parent(tabs_entity);
                }
            });
    }

    fn set_family_mode(
        mut family_mode: ResMut<NextState<FamilyMode>>,
        buttons: Query<(Ref<Toggled>, &FamilyMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                info!("changing family mode to `{mode:?}`");
                family_mode.set(mode);
            }
        }
    }

    fn update_task_nodes(
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

    fn cancel_task(
        mut cancel_events: EventWriter<TaskCancel>,
        mut click_events: EventReader<Click>,
        buttons: Query<&ButtonTask>,
    ) {
        for button_task in buttons.iter_many(click_events.read().map(|event| event.0)) {
            cancel_events.send(TaskCancel(button_task.0));
        }
    }

    fn cleanup_tasks(
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

    fn update_budget(
        families: Query<&Budget, (With<SelectedFamily>, Changed<Budget>)>,
        mut labels: Query<&mut Text, With<BudgetLabel>>,
    ) {
        if let Ok(budget) = families.get_single() {
            debug!("changing budget to `{budget:?}`");
            labels.single_mut().sections[0].value = budget.to_string();
        }
    }

    fn select_actor(
        mut commands: Commands,
        actor_buttons: Query<(Ref<Toggled>, &PlayActor), Changed<Toggled>>,
    ) {
        for (toggled, play_actor) in &actor_buttons {
            if toggled.0 && !toggled.is_added() {
                commands.entity(play_actor.0).insert(SelectedActor);
            }
        }
    }

    fn update_need_bars(
        mut commands: Commands,
        theme: Res<Theme>,
        needs: Query<(Entity, &NeedGlyph, Ref<Need>)>,
        actors: Query<(&Children, Ref<SelectedActor>)>,
        tabs: Query<(&TabContent, &InfoTab)>,
        mut progress_bars: Query<(&mut ProgressBar, &BarNeed)>,
    ) {
        let (children, selected_actor) = actors.single();
        let (tab_content, _) = tabs
            .iter()
            .find(|(_, &tab)| tab == InfoTab::Needs)
            .expect("tab with cities should be spawned on state enter");

        if selected_actor.is_added() {
            commands.entity(tab_content.0).despawn_descendants();
        }

        for (entity, glyph, need) in needs
            .iter_many(children)
            .filter(|(.., need)| need.is_changed() || selected_actor.is_added())
        {
            if let Some((mut progress_bar, _)) = progress_bars
                .iter_mut()
                .find(|(_, bar_need)| bar_need.0 == entity)
            {
                trace!("updating bar with `{need:?}` for `{entity}`");
                progress_bar.0 = need.0;
            } else {
                trace!("creating bar with `{need:?}` for `{entity}`");
                commands.entity(tab_content.0).with_children(|parent| {
                    parent.spawn(LabelBundle::symbol(&theme, glyph.0));
                    parent.spawn((BarNeed(entity), ProgressBarBundle::new(&theme, need.0)));
                });
            }
        }
    }

    fn cleanup_need_bars(
        mut commands: Commands,
        mut removed_needs: RemovedComponents<Need>,
        progress_bars: Query<(Entity, &BarNeed)>,
    ) {
        for need_entity in removed_needs.read() {
            if let Some((bar_entity, _)) = progress_bars
                .iter()
                .find(|(_, bar_need)| bar_need.0 == need_entity)
            {
                debug!("despawning bar for need `{need_entity}`");
                commands.entity(bar_entity).despawn_recursive();
            }
        }
    }

    fn set_building_mode(
        mut building_mode: ResMut<NextState<BuildingMode>>,
        buttons: Query<(Ref<Toggled>, &BuildingMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                info!("changing building mode to `{mode:?}`");
                building_mode.set(mode);
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

fn setup_portrait_node(parent: &mut ChildBuilder, theme: &Theme, budget: Budget) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(180.0),
                height: Val::Px(30.0),
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
    members: &FamilyMembers,
    active_entity: Entity,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                column_gap: theme.gap.normal,
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for &entity in members.iter() {
                parent.spawn((
                    PlayActor(entity),
                    Preview::Actor(entity),
                    ExclusiveButton,
                    Toggled(entity == active_entity),
                    ImageButtonBundle::placeholder(theme),
                ));
            }
        });
}

fn setup_info_node(parent: &mut ChildBuilder, tab_commands: &mut Commands, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::ColumnReverse,
                position_type: PositionType::Absolute,
                align_self: AlignSelf::FlexEnd,
                right: Val::Px(0.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            let tabs_entity = parent
                .spawn(NodeBundle {
                    style: Style {
                        padding: theme.padding.normal,
                        align_self: AlignSelf::FlexEnd,
                        ..Default::default()
                    },
                    background_color: theme.panel_color.into(),
                    ..Default::default()
                })
                .id();

            for (index, tab) in InfoTab::iter().enumerate() {
                let tab_content = match tab {
                    InfoTab::Needs => parent
                        .spawn(NodeBundle {
                            style: Style {
                                display: Display::Grid,
                                width: Val::Px(400.0),
                                column_gap: theme.gap.normal,
                                row_gap: theme.gap.normal,
                                padding: theme.padding.normal,
                                grid_template_columns: vec![
                                    GridTrack::auto(),
                                    GridTrack::flex(1.0),
                                    GridTrack::auto(),
                                    GridTrack::flex(1.0),
                                ],
                                ..Default::default()
                            },
                            background_color: theme.panel_color.into(),

                            ..Default::default()
                        })
                        .id(),
                    InfoTab::Skills => parent.spawn(NodeBundle::default()).id(),
                };

                tab_commands
                    .spawn((
                        tab,
                        TabContent(tab_content),
                        ExclusiveButton,
                        Toggled(index == 0),
                        TextButtonBundle::symbol(theme, tab.glyph()),
                    ))
                    .set_parent(tabs_entity);
            }
        });
}

fn setup_building_hud(
    parent: &mut ChildBuilder,
    tab_commands: &mut Commands,
    theme: &Theme,
    objects_info: &Assets<ObjectInfo>,
) {
    let tabs_entity = parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::FlexEnd,
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .id();

    for mode in BuildingMode::iter() {
        let content_entity = parent
            .spawn(NodeBundle {
                style: Style {
                    align_self: AlignSelf::FlexEnd,
                    padding: theme.padding.normal,
                    column_gap: theme.gap.normal,
                    ..Default::default()
                },
                background_color: theme.panel_color.into(),
                ..Default::default()
            })
            .with_children(|parent| match mode {
                BuildingMode::Objects => {
                    objects_node::setup_objects_node(
                        parent,
                        tab_commands,
                        theme,
                        objects_info,
                        ObjectCategory::FAMILY_CATEGORIES,
                    );
                }
                BuildingMode::Walls => setup_walls_node(parent, theme),
            })
            .id();

        tab_commands
            .spawn((
                mode,
                TabContent(content_entity),
                ExclusiveButton,
                Toggled(mode == Default::default()),
                TextButtonBundle::symbol(theme, mode.glyph()),
            ))
            .set_parent(tabs_entity);
    }
}

fn setup_walls_node(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            // Just a stab for instruments.
            parent.spawn((
                ExclusiveButton,
                Toggled(true),
                TextButtonBundle::symbol(theme, "➕"),
            ));
        });
}

#[derive(Component)]
struct ActiveTasksNode;

#[derive(Component)]
struct QueuedTasksNode;

#[derive(Component, Debug)]
struct ButtonTask(Entity);

#[derive(Component)]
struct BudgetLabel;

#[derive(Component, Debug)]
struct PlayActor(Entity);

#[derive(Component)]
struct BarNeed(Entity);

#[derive(Component, EnumIter, Clone, Copy, PartialEq)]
enum InfoTab {
    Needs,
    Skills,
}

impl InfoTab {
    fn glyph(self) -> &'static str {
        match self {
            InfoTab::Needs => "📈",
            InfoTab::Skills => "💡",
        }
    }
}
