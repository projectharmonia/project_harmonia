use std::{any, marker::PhantomData};

use bevy::{asset::Asset, prelude::*, scene::SceneInstance};
use iyes_loopless::prelude::IntoConditionalSystem;

use super::game_world::GameWorld;

/// Makes a deep copy of all assets `T` on entity's children with component [`UniqueAsset<T>`].
///
/// Used to modify assets without changing the original asset on other entities.
#[derive(Default)]
pub(super) struct UniqueAssetPlugin<T>(PhantomData<T>);

impl<T: Asset + Clone> Plugin for UniqueAssetPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_system(Self::init_system.run_if_resource_exists::<GameWorld>());
    }
}

impl<T: Asset + Clone> UniqueAssetPlugin<T> {
    fn init_system(
        mut commmands: Commands,
        mut assets: ResMut<Assets<T>>,
        scene_spawner: Res<SceneSpawner>,
        unique_assets: Query<(Entity, &SceneInstance), With<UniqueAsset<T>>>,
        children: Query<&Children>,
        handles: Query<&Handle<T>>,
    ) {
        for parent_entity in unique_assets.iter().filter_map(|(entity, scene_instance)| {
            scene_spawner
                .instance_is_ready(**scene_instance)
                .then_some(entity)
        }) {
            for child_entity in children.iter_descendants(parent_entity) {
                if let Ok(original_handle) = handles.get(child_entity) {
                    let asset = assets
                        .get(original_handle)
                        .expect("asset should be loaded after scene loading")
                        .clone();
                    let unique_handle = assets.add(asset);
                    commmands.entity(child_entity).insert(unique_handle);
                }
            }
            debug!(
                "unique {} assets was assigned for {parent_entity:?} children",
                any::type_name::<T>()
            );
            commmands.entity(parent_entity).remove::<UniqueAsset<T>>();
        }
    }
}

#[derive(Component, Default)]
pub(super) struct UniqueAsset<T>(PhantomData<T>);
