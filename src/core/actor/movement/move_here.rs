use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

use super::{Movement, MovementBundle};
use crate::core::{
    actor::Actor,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    game_world::WorldState,
    ground::Ground,
    task::{ReflectTask, Task, TaskGroups, TaskList},
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<MoveHere>()
            .register_component_as::<dyn Task, MoveHere>()
            .add_system(
                Self::tasks_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::init_system,
                    Self::cancellation_system,
                    Self::finish_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
            );
    }
}

impl MoveHerePlugin {
    fn tasks_system(
        mut grounds: Query<(&CursorHover, &mut TaskList), (With<Ground>, Added<TaskList>)>,
    ) {
        if let Ok((hover, mut task_list)) = grounds.get_single_mut() {
            task_list.push(Box::new(MoveHere {
                destination: hover.0,
                movement: Movement::Walk,
            }));
        }
    }

    fn init_system(mut commands: Commands, actors: Query<(Entity, &MoveHere), Added<MoveHere>>) {
        for (entity, move_here) in &actors {
            commands.entity(entity).insert(MovementBundle::new(
                move_here.movement,
                move_here.destination,
            ));
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        mut removed_tasks: RemovedComponents<MoveHere>,
        actors: Query<(), With<Actor>>,
    ) {
        for entity in &mut removed_tasks {
            if actors.get(entity).is_ok() {
                commands.entity(entity).remove::<MovementBundle>();
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut removed_movements: RemovedComponents<Movement>,
        actors: Query<Ref<MoveHere>>,
    ) {
        for entity in &mut removed_movements {
            if let Ok(move_here) = actors.get(entity) {
                if !move_here.is_added() {
                    commands.entity(entity).remove::<MoveHere>();
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component, Task)]
struct MoveHere {
    destination: Vec3,
    movement: Movement,
}

impl Task for MoveHere {
    fn name(&self) -> &str {
        match self.movement {
            Movement::Walk => "Walk here",
        }
    }

    fn groups(&self) -> TaskGroups {
        TaskGroups::LEGS
    }
}
