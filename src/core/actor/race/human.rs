use bevy::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use super::{Race, RaceExt};
use crate::core::{
    actor::{
        animation::ActorAnimation,
        needs::{Bladder, Energy, Fun, Hunger, Hygiene, Social},
        Actor, Sex,
    },
    asset_handles::{AssetCollection, AssetHandles},
    game_world::WorldState,
    reflect_bundle::ReflectBundle,
};

pub(super) struct HumanPlugin;

impl Plugin for HumanPlugin {
    fn build(&self, app: &mut App) {
        app.register_race::<Human, HumanBundle>()
            .init_resource::<AssetHandles<HumanScene>>()
            .add_systems(
                (Self::init_system, Self::init_mesh_system).in_set(OnUpdate(WorldState::InWorld)),
            );
    }
}

impl HumanPlugin {
    fn init_system(
        mut commands: Commands,
        actor_animations: Res<AssetHandles<ActorAnimation>>,
        actors: Query<Entity, (Added<Human>, With<Actor>)>,
    ) {
        for entity in &actors {
            commands.entity(entity).insert((
                actor_animations.handle(ActorAnimation::Idle),
                VisibilityBundle::default(),
                GlobalTransform::default(),
            ));
        }
    }

    /// Separated in order to be triggered in family editor too.
    fn init_mesh_system(
        mut commands: Commands,
        human_scenes: Res<AssetHandles<HumanScene>>,
        actors: Query<(Entity, &Sex), (Changed<Sex>, With<Human>)>,
    ) {
        for (entity, &sex) in &actors {
            commands
                .entity(entity)
                .insert(human_scenes.handle(sex.into()))
                .despawn_descendants();
        }
    }
}

/// Components for a actor inside the game.
#[derive(Bundle, Default, Reflect)]
#[reflect(Bundle)]
struct HumanBundle {
    human: Human,
    hunger: Hunger,
    social: Social,
    hygiene: Hygiene,
    fun: Fun,
    energy: Energy,
    bladder: Bladder,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub(crate) struct Human;

impl Race for Human {
    fn glyph(&self) -> &'static str {
        "👤"
    }
}

#[derive(Clone, Copy, Debug, IntoPrimitive, EnumIter, Default)]
#[repr(usize)]
enum HumanScene {
    #[default]
    Male,
    Female,
}

impl AssetCollection for HumanScene {
    type AssetType = Scene;

    fn asset_path(&self) -> &'static str {
        match self {
            Self::Male => "base/actors/bot/y_bot/y_bot.gltf#Scene0",
            Self::Female => "base/actors/bot/x_bot/x_bot.gltf#Scene0",
        }
    }
}

impl From<Sex> for HumanScene {
    fn from(value: Sex) -> Self {
        match value {
            Sex::Male => Self::Male,
            Sex::Female => Self::Female,
        }
    }
}
