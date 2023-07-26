use bevy::{prelude::*, window::PrimaryWindow};
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::{
    core::{
        action::Action,
        actor::{
            task::{Task, TaskList, TaskListSet, TaskRequest},
            ActiveActor,
        },
        cursor_hover::CursorHover,
        family::FamilyMode,
        game_state::GameState,
    },
    ui::{
        theme::Theme,
        widget::{button::TextButtonBundle, click::Click, ui_root::UiRoot, LabelBundle},
    },
};

pub(super) struct TaskMenuPlugin;

impl Plugin for TaskMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::button_system,
                Self::cleanup_system.run_if(
                    action_just_pressed(Action::Cancel)
                        .or_else(on_event::<TaskList>())
                        .or_else(on_event::<TaskRequest>()),
                ),
                Self::setup_system.after(TaskListSet),
            )
                .chain()
                .run_if(in_state(GameState::Family))
                .run_if(in_state(FamilyMode::Life)),
        );
    }
}

impl TaskMenuPlugin {
    fn setup_system(
        mut commands: Commands,
        mut list_events: ResMut<Events<TaskList>>,
        theme: Res<Theme>,
        hovered: Query<&Name, With<CursorHover>>,
        windows: Query<&Window, With<PrimaryWindow>>,
        roots: Query<Entity, With<UiRoot>>,
    ) {
        let tasks = list_events.drain().map(|event| event.0).collect::<Vec<_>>();
        if tasks.is_empty() {
            return;
        }

        let primary_window = windows.single();
        let cursor_position = primary_window.cursor_position().unwrap_or_default();
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn_empty()
                .with_children(|parent| {
                    parent.spawn(LabelBundle::normal(&theme, hovered.single()));

                    for (index, task) in tasks.iter().enumerate() {
                        parent.spawn((
                            TaskMenuIndex(index),
                            TextButtonBundle::normal(&theme, task.name()),
                        ));
                    }
                })
                .insert((
                    TaskMenu(tasks),
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Val::Px(cursor_position.x),
                            top: Val::Px(cursor_position.y),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            padding: theme.padding.normal,
                            row_gap: theme.gap.normal,
                            ..Default::default()
                        },
                        background_color: theme.panel_color.into(),
                        ..Default::default()
                    },
                ));
        });
    }

    fn button_system(
        mut task_events: EventWriter<TaskRequest>,
        mut click_events: EventReader<Click>,
        buttons: Query<&TaskMenuIndex>,
        mut task_menus: Query<&mut TaskMenu>,
        active_actors: Query<Entity, With<ActiveActor>>,
    ) {
        for event in &mut click_events {
            if let Ok(task_index) = buttons.get(event.0) {
                let mut task_menu = task_menus.single_mut();
                let task = task_menu.swap_remove(task_index.0);

                task_events.send(TaskRequest {
                    entity: active_actors.single(),
                    task,
                });
            }
        }
    }

    fn cleanup_system(mut commands: Commands, task_menus: Query<Entity, With<TaskMenu>>) {
        if let Ok(entity) = task_menus.get_single() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

#[derive(Component, Deref, DerefMut)]
pub(crate) struct TaskMenu(Vec<Box<dyn Task>>);

#[derive(Component)]
struct TaskMenuIndex(usize);
