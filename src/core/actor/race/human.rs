use bevy::prelude::*;
use bevy_mod_outline::OutlineBundle;
use bevy_rapier3d::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use super::{AppRaceExt, Race, ReflectRace};
use crate::core::{
    actor::{
        needs::{Bladder, Energy, Fun, Hunger, Hygiene, Social},
        Actor, ActorAnimation, Sex,
    },
    asset_handles::{AssetCollection, AssetHandles},
    collision_groups::LifescapeGroupsExt,
    cursor_hover::{Hoverable, OutlineHoverExt},
    game_world::WorldState,
    ready_scene::ReadyScene,
};

pub(super) struct HumanPlugin;

impl Plugin for HumanPlugin {
    fn build(&self, app: &mut App) {
        app.register_race::<Human>()
            .init_resource::<AssetHandles<HumanScene>>()
            .add_systems(
                (
                    Self::init_system,
                    Self::init_mesh_system,
                    Self::scene_init_system,
                )
                    .in_set(OnUpdate(WorldState::InWorld)),
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
            const HALF_HEIGHT: f32 = 0.6;
            const RADIUS: f32 = 0.3;
            commands
                .entity(entity)
                .insert((
                    HumanBundle::default(),
                    actor_animations.handle(ActorAnimation::Idle),
                    VisibilityBundle::default(),
                    GlobalTransform::default(),
                    Hoverable,
                ))
                .with_children(|parent| {
                    parent.spawn((
                        SpatialBundle::from_transform(Transform::from_translation(
                            Vec3::Y * (HALF_HEIGHT + RADIUS),
                        )),
                        CollisionGroups::new(Group::ACTOR, Group::ALL),
                        Collider::capsule_y(HALF_HEIGHT, RADIUS),
                    ));
                });
        }
    }

    fn scene_init_system(
        mut commands: Commands,
        actors: Query<Entity, (Added<ReadyScene>, With<Human>)>,
        chidlren: Query<&Children>,
        meshes: Query<(), With<Handle<Mesh>>>,
    ) {
        for actor_entity in &actors {
            for child_entity in chidlren
                .iter_descendants(actor_entity)
                .filter(|&entity| meshes.get(entity).is_ok())
            {
                commands.entity(child_entity).insert(OutlineBundle::hover());
            }
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
                .insert(human_scenes.handle(sex.into()));
        }
    }
}

/// Components specific to the human race.
#[derive(Bundle, Default)]
struct HumanBundle {
    hunger: Hunger,
    social: Social,
    hygiene: Hygiene,
    fun: Fun,
    energy: Energy,
    bladder: Bladder,
}

#[derive(Component, Reflect, Default)]
#[reflect(Component, Default, Race)]
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
