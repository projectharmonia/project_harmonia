use bevy::{asset::AssetPath, ecs::reflect::ReflectBundle, prelude::*};
use bevy_replicon::prelude::*;
use num_enum::IntoPrimitive;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

use super::{
    needs::{Bladder, Energy, Fun, Hunger, Hygiene, Need, Social},
    FirstName, LastName, Sex,
};
use crate::{
    asset::collection::{AssetCollection, Collection},
    game_world::family::editor::{
        ActorBundle, EditorFirstName, EditorLastName, EditorSex, FamilyScene, ReflectActorBundle,
    },
};

pub(super) struct HumanPlugin;

impl Plugin for HumanPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Human>()
            .replicate::<Human>()
            .register_type::<HumanBundle>()
            .init_resource::<Collection<HumanScene>>()
            .add_observer(Self::init_needs)
            .add_systems(
                Update,
                (Self::update_sex::<EditorSex>, Self::update_sex::<Sex>),
            )
            .add_systems(
                PostUpdate,
                Self::fill_scene.run_if(resource_added::<FamilyScene>),
            );
    }
}

impl HumanPlugin {
    fn init_needs(
        trigger: Trigger<OnAdd, Children>,
        mut commands: Commands,
        actors: Query<&Children, With<Human>>,
        need: Query<(), With<Need>>,
    ) {
        let Ok(children) = actors.get(trigger.entity()) else {
            return;
        };

        if need.iter_many(children).next().is_none() {
            debug!("initializing human needs `{}`", trigger.entity());
            commands.entity(trigger.entity()).with_children(|parent| {
                parent.spawn(Bladder);
                parent.spawn(Energy);
                parent.spawn(Fun);
                parent.spawn(Hunger);
                parent.spawn(Hygiene);
                parent.spawn(Social);
            });
        }
    }

    fn update_sex<C: Component + Into<HumanScene> + Copy>(
        human_scenes: Res<Collection<HumanScene>>,
        mut actors: Query<(Entity, &C, &mut SceneRoot), Changed<C>>,
    ) {
        for (entity, &sex, mut scene_root) in &mut actors {
            debug!("initializing sex for human `{entity}`");
            **scene_root = human_scenes.handle(sex.into());
        }
    }

    /// Fills [`FamilyScene`] with editing human actors.
    fn fill_scene(
        mut family_scene: ResMut<FamilyScene>,
        actors: Query<(&EditorFirstName, &EditorLastName, &EditorSex), With<EditorHuman>>,
    ) {
        for (first_name, last_name, &sex) in &actors {
            debug!(
                "adding human '{} {}' to family scene '{}'",
                first_name.0, last_name.0, family_scene.name
            );
            family_scene.actors.push(Box::new(HumanBundle {
                first_name: first_name.clone().into(),
                last_name: last_name.clone().into(),
                sex: sex.into(),
                human: Human,
            }));
        }
    }
}

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct Human;

#[derive(Component, Default)]
pub(crate) struct EditorHuman;

#[derive(Bundle, Default, Reflect)]
#[reflect(Bundle, ActorBundle)]
struct HumanBundle {
    first_name: FirstName,
    last_name: LastName,
    sex: Sex,
    human: Human,
}

impl ActorBundle for HumanBundle {
    fn glyph(&self) -> &'static str {
        "👤"
    }
}

#[derive(Clone, Copy, IntoPrimitive, EnumIter, Default)]
#[repr(usize)]
enum HumanScene {
    #[default]
    Male,
    Female,
}

impl AssetCollection for HumanScene {
    type AssetType = Scene;

    fn asset_path(&self) -> AssetPath<'static> {
        match self {
            Self::Male => GltfAssetLabel::Scene(0).from_asset("base/actors/bot/y_bot/y_bot.gltf"),
            Self::Female => GltfAssetLabel::Scene(0).from_asset("base/actors/bot/x_bot/x_bot.gltf"),
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

impl From<EditorSex> for HumanScene {
    fn from(value: EditorSex) -> Self {
        match value {
            EditorSex::Male => Self::Male,
            EditorSex::Female => Self::Female,
        }
    }
}
