use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

use super::{animation::HumanAnimation, Sex};
use crate::core::{
    asset_handles::AssetHandles,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    ground::Ground,
    task::{ReflectTask, Task, TaskGroups, TaskList},
};

pub(super) struct MovementPlugin;

impl Plugin for MovementPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Walk>()
            .register_component_as::<dyn Task, Walk>()
            .add_system(
                Self::tasks_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_systems(
                (
                    Self::init_system,
                    Self::movement_system,
                    Self::cleanup_system,
                )
                    .in_set(ServerSet::Authority),
            );
    }
}

impl MovementPlugin {
    fn tasks_system(
        mut commands: Commands,
        grounds: Query<(Entity, &CursorHover), (With<Ground>, Added<TaskList>)>,
    ) {
        if let Ok((entity, hover)) = grounds.get_single() {
            commands.entity(entity).with_children(|parent| {
                parent.spawn(Walk(hover.0));
            });
        }
    }

    fn init_system(
        human_animations: Res<AssetHandles<HumanAnimation>>,
        mut actors: Query<(&mut Handle<AnimationClip>, &Sex), Added<Walk>>,
    ) {
        for (mut anim_handle, sex) in &mut actors {
            let walk_anim = match sex {
                Sex::Male => HumanAnimation::MaleWalk,
                Sex::Female => HumanAnimation::FemaleWalk,
            };
            *anim_handle = human_animations.handle(walk_anim);
        }
    }

    fn movement_system(
        mut commands: Commands,
        time: Res<Time>,
        human_animations: Res<AssetHandles<HumanAnimation>>,
        mut actors: Query<(Entity, &mut Transform, &mut Handle<AnimationClip>, &Walk)>,
    ) {
        for (entity, mut transform, mut anim_handle, walk) in &mut actors {
            let direction = walk.0 - transform.translation;

            if direction.length() < 0.1 {
                commands.entity(entity).remove::<Walk>();
                *anim_handle = human_animations.handle(HumanAnimation::Idle);
            } else {
                const ROTATION_SPEED: f32 = 10.0;
                const WALK_SPEED: f32 = 2.0;
                let delta_secs = time.delta_seconds();
                let target_rotation = transform.looking_to(direction, Vec3::Y).rotation;

                transform.translation += direction.normalize() * WALK_SPEED * delta_secs;
                transform.rotation = transform
                    .rotation
                    .slerp(target_rotation, ROTATION_SPEED * delta_secs);
            }
        }
    }

    fn cleanup_system(
        mut stopped_walking: RemovedComponents<Walk>,
        human_animations: Res<AssetHandles<HumanAnimation>>,
        mut actors: Query<&mut Handle<AnimationClip>>,
    ) {
        for entity in &mut stopped_walking {
            if let Ok(mut anim_handle) = actors.get_mut(entity) {
                *anim_handle = human_animations.handle(HumanAnimation::Idle);
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component, Task)]
pub(crate) struct Walk(pub(crate) Vec3);

impl Task for Walk {
    fn name(&self) -> &'static str {
        "Walk"
    }

    fn groups(&self) -> TaskGroups {
        TaskGroups::LEGS
    }
}
