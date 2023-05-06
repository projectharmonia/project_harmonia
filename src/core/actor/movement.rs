mod walk;

use std::sync::{Arc, RwLock};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use futures_lite::future;
use oxidized_navigation::{query, tiles::NavMeshTiles, NavMeshSettings};

use super::{animation::ActorAnimation, Sex};
use crate::core::{asset_handles::AssetHandles, game_world::WorldState};
use walk::WalkPlugin;

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(WalkPlugin).add_systems(
            (
                Self::poll_system,
                Self::movement_system,
                Self::cleanup_system,
            )
                .in_set(OnUpdate(WorldState::InWorld)),
        );
    }
}

impl MovementPlugin {
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

    fn cleanup_system(
        mut removed_paths: RemovedComponents<MovePath>,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut actors: Query<&mut Handle<AnimationClip>>,
    ) {
        for entity in &mut removed_paths {
            if let Ok(mut anim_handle) = actors.get_mut(entity) {
                *anim_handle = actor_animations.handle(ActorAnimation::Idle);
            }
        }
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
