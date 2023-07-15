use std::mem;

use bevy::prelude::*;
use bevy_replicon::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use super::{RaceBundle, ReflectRaceBundle};
use crate::core::{
    actor::{
        needs::{Bladder, Energy, Fun, Hunger, Hygiene, NeedBundle, Social},
        Actor, FirstName, LastName, Sex,
    },
    asset_handles::{AssetCollection, AssetHandles},
    family::{
        editor::{EditableActor, EditorPlugin},
        family_spawn::FamilyScene,
    },
    game_state::GameState,
    game_world::WorldState,
    reflect_bundle::ReflectBundle,
};

pub(super) struct HumanPlugin;

impl Plugin for HumanPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Human>()
            .register_type::<HumanRaceBundle>()
            .init_resource::<AssetHandles<HumanScene>>()
            .add_system(
                Self::scene_setup_system
                    .before(EditorPlugin::scene_save_system)
                    .in_set(OnUpdate(GameState::FamilyEditor)),
            )
            .add_systems(
                (Self::init_system, Self::needs_init_system).in_set(OnUpdate(WorldState::InWorld)),
            );
    }
}

impl HumanPlugin {
    fn init_system(
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

    fn needs_init_system(
        mut commands: Commands,
        actors: Query<Entity, (Added<Human>, With<Actor>, Without<Children>)>,
    ) {
        for entity in &actors {
            commands.entity(entity).with_children(|parent| {
                parent.spawn(NeedBundle::<Bladder>::default());
                parent.spawn(NeedBundle::<Energy>::default());
                parent.spawn(NeedBundle::<Fun>::default());
                parent.spawn(NeedBundle::<Hunger>::default());
                parent.spawn(NeedBundle::<Hygiene>::default());
                parent.spawn(NeedBundle::<Social>::default());
            });
        }
    }

    /// Fills [`FamilyScene`] with editing human actors.
    fn scene_setup_system(
        mut family_scenes: Query<&mut FamilyScene, Added<FamilyScene>>,
        mut actors: Query<(&mut FirstName, &mut LastName, &Sex), With<EditableActor>>,
    ) {
        if let Ok(mut family_scene) = family_scenes.get_single_mut() {
            for (mut first_name, mut last_name, &sex) in &mut actors {
                family_scene.actors.push(Box::new(HumanRaceBundle::new(
                    mem::take(&mut first_name),
                    mem::take(&mut last_name),
                    sex,
                )));
            }
        }
    }
}

#[derive(Component, Reflect, Default, Debug)]
#[reflect(Component)]
pub(crate) struct Human;

#[derive(Bundle, Debug, Default, Reflect)]
#[reflect(Bundle, RaceBundle)]
struct HumanRaceBundle {
    first_name: FirstName,
    last_name: LastName,
    sex: Sex,
    human: Human,
}

impl HumanRaceBundle {
    fn new(first_name: FirstName, last_name: LastName, sex: Sex) -> Self {
        Self {
            first_name,
            last_name,
            sex,
            human: Human,
        }
    }
}

impl RaceBundle for HumanRaceBundle {
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
