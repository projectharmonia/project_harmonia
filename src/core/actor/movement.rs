use std::sync::{Arc, RwLock};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;
use futures_lite::future;
use oxidized_navigation::{query, tiles::NavMeshTiles, NavMesh, NavMeshSettings};
use serde::{Deserialize, Serialize};

use super::{animation::ActorAnimation, Actor, Sex};
use crate::core::{
    asset_handles::AssetHandles,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    game_world::WorldState,
    ground::Ground,
    task::{ReflectTask, Task, TaskGroups, TaskList},
};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
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
                    Self::poll_system,
                    Self::movement_system,
                    Self::cancellation_system,
                    Self::cleanup_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
            );
    }
}

impl MovementPlugin {
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

    fn poll_system(
        mut commands: Commands,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<(Entity, &Sex, &mut ComputePath, &mut Handle<AnimationClip>)>,
    ) {
        for (entity, sex, mut compute_path, mut anim_handle) in &mut actors {
            if let Some(mut path) = future::block_on(future::poll_once(&mut compute_path.0)) {
                let walk_anim = match sex {
                    Sex::Male => ActorAnimation::MaleWalk,
                    Sex::Female => ActorAnimation::FemaleWalk,
                };
                *anim_handle = actor_animations.handle(walk_anim);
                path.reverse();
                path.pop(); // Drop current position.
                commands
                    .entity(entity)
                    .insert(MovePath(path))
                    .remove::<ComputePath>();
            }
        }
    }

    fn movement_system(
        mut commands: Commands,
        time: Res<Time>,
        mut actors: Query<(Entity, &mut Transform, &mut MovePath)>,
    ) {
        for (entity, mut transform, mut move_path) in &mut actors {
            if let Some(&waypoint) = move_path.last() {
                const ROTATION_SPEED: f32 = 10.0;
                const WALK_SPEED: f32 = 2.0;
                let direction = waypoint - transform.translation;
                let delta_secs = time.delta_seconds();
                let target_rotation = transform.looking_to(direction, Vec3::Y).rotation;

                transform.translation += direction.normalize() * WALK_SPEED * delta_secs;
                transform.rotation = transform
                    .rotation
                    .slerp(target_rotation, ROTATION_SPEED * delta_secs);

                if direction.length() < 0.1 {
                    move_path.pop();
                }
            } else {
                commands.entity(entity).remove::<MovePath>();
            }
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

    fn cleanup_system(
        mut commands: Commands,
        mut removed_paths: RemovedComponents<MovePath>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<&mut Handle<AnimationClip>>,
    ) {
        for entity in &mut removed_paths {
            if let Ok(mut anim_handle) = actors.get_mut(entity) {
                commands.entity(entity).remove::<Walk>();
                *anim_handle = actor_animations.handle(ActorAnimation::Idle);
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

#[derive(Component)]
struct ComputePath(bevy::tasks::Task<Vec<Vec3>>);

impl ComputePath {
    fn new(
        tiles: Arc<RwLock<NavMeshTiles>>,
        settings: NavMeshSettings,
        start: Vec3,
        end: Vec3,
    ) -> Self {
        let thread_pool = AsyncComputeTaskPool::get();
        let task = thread_pool.spawn(async move {
            let tiles = tiles.read().expect("tiles shouldn't be poisoned");
            let path = query::find_path(&tiles, &settings, start, end, None, None)
                .expect("navigation should happen only inside the city");

            query::perform_string_pulling_on_path(&tiles, start, end, &path)
                .expect("passed tiles should be valid and connected")
        });

        Self(task)
    }
}

#[derive(Component, Deref, DerefMut)]
pub(crate) struct MovePath(pub(crate) Vec<Vec3>);
