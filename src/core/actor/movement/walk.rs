use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;
use oxidized_navigation::{NavMesh, NavMeshSettings};
use serde::{Deserialize, Serialize};

use super::{ComputePath, MovePath};
use crate::core::{
    actor::Actor,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    game_world::WorldState,
    ground::Ground,
    task::{ReflectTask, Task, TaskGroups, TaskList},
};

pub(super) struct WalkPlugin;

impl Plugin for WalkPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Walk>()
            .register_component_as::<dyn Task, Walk>()
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

impl WalkPlugin {
    fn tasks_system(
        mut grounds: Query<(&CursorHover, &mut TaskList), (With<Ground>, Added<TaskList>)>,
    ) {
        if let Ok((hover, mut task_list)) = grounds.get_single_mut() {
            task_list.push(Box::new(Walk(hover.0)));
        }
    }

    fn init_system(
        mut commands: Commands,
        nav_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        actors: Query<(Entity, &Transform, &Walk), Added<Walk>>,
    ) {
        for (entity, transform, walk) in &actors {
            commands.entity(entity).insert(ComputePath::new(
                nav_mesh.get(),
                nav_settings.clone(),
                transform.translation,
                walk.0,
            ));
        }
    }

    fn cancellation_system(
        mut commands: Commands,
        mut removed_walks: RemovedComponents<Walk>,
        actors: Query<(), With<Actor>>,
    ) {
        for entity in &mut removed_walks {
            if actors.get(entity).is_ok() {
                commands
                    .entity(entity)
                    .remove::<ComputePath>()
                    .remove::<MovePath>();
            }
        }
    }

    fn finish_system(
        mut commands: Commands,
        mut removed_paths: RemovedComponents<MovePath>,
        actors: Query<Ref<Walk>>,
    ) {
        for entity in &mut removed_paths {
            if let Ok(walk) = actors.get(entity) {
                if !walk.is_added() {
                    commands.entity(entity).remove::<Walk>();
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component, Task)]
struct Walk(Vec3);

impl Task for Walk {
    fn name(&self) -> &str {
        "Walk"
    }

    fn groups(&self) -> TaskGroups {
        TaskGroups::LEGS
    }
}
