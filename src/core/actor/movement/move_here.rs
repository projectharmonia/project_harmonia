use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use super::{Movement, MovementBundle};
use crate::core::{
    actor::ActorAnimation,
    asset_handles::AssetHandles,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    game_world::WorldState,
    ground::Ground,
    navigation::{endpoint::Endpoint, Navigation},
    task::{ActiveTask, AppTaskExt, CancelledTask, ListedTask, TaskGroups, TaskList},
};

pub(super) struct MoveHerePlugin;

impl Plugin for MoveHerePlugin {
    fn build(&self, app: &mut App) {
        app.register_task::<MoveHere>()
            .add_system(
                Self::list_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::init_system,
                    Self::activation_system,
                    Self::cancellation_system,
                    Self::finish_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
            );
    }
}

impl MoveHerePlugin {
    fn list_system(
        mut commands: Commands,
        grounds: Query<(Entity, &CursorHover), (With<Ground>, Added<TaskList>)>,
    ) {
        if let Ok((entity, hover)) = grounds.get_single() {
            commands.entity(entity).with_children(|parent| {
                parent.spawn((
                    ListedTask,
                    MoveHere {
                        endpoint: hover.0,
                        movement: Movement::Walk,
                    },
                ));
                parent.spawn((
                    ListedTask,
                    MoveHere {
                        endpoint: hover.0,
                        movement: Movement::Run,
                    },
                ));
            });
        }
    }

    fn init_system(mut commands: Commands, tasks: Query<(Entity, &MoveHere), Added<MoveHere>>) {
        for (entity, move_here) in &tasks {
            let name = match move_here.movement {
                Movement::Walk => "Walk",
                Movement::Run => "Run",
            };
            commands
                .entity(entity)
                .insert((Name::new(name), TaskGroups::LEGS));
        }
    }

    fn activation_system(
        mut commands: Commands,
        tasks: Query<(&Parent, &MoveHere), Added<ActiveTask>>,
    ) {
        for (parent, move_here) in &tasks {
            commands.entity(**parent).insert((
                MovementBundle::new(move_here.movement),
                Endpoint::new(move_here.endpoint),
            ));
        }
    }

    fn cancellation_system(mut commands: Commands, tasks: Query<&Parent, Added<CancelledTask>>) {
        for parent in &tasks {
            commands.entity(**parent).remove::<Navigation>();
        }
    }

    fn finish_system(
        mut commands: Commands,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        mut removed_movements: RemovedComponents<Movement>,
        mut actors: Query<(&Children, &mut Handle<AnimationClip>)>,
        tasks: Query<Entity, (With<MoveHere>, With<ActiveTask>)>,
    ) {
        for actor_entity in &mut removed_movements {
            if let Ok((children, mut animation_handle)) = actors.get_mut(actor_entity) {
                if let Some(task_entity) = tasks.iter_many(children).next() {
                    commands.entity(task_entity).despawn();
                    *animation_handle = actor_animations.handle(ActorAnimation::Idle);
                }
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
struct MoveHere {
    endpoint: Vec3,
    movement: Movement,
}
