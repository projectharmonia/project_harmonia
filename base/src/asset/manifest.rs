pub mod object_manifest;
pub mod road_manifest;

use std::{env, path::Path};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use walkdir::WalkDir;

use crate::core::GameState;
use object_manifest::{ObjectLoader, ObjectManifest};
use road_manifest::{RoadLoader, RoadManifest};

pub(super) struct ManifestPlugin;

impl Plugin for ManifestPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ObjectManifest>()
            .init_asset::<RoadManifest>()
            .init_asset_loader::<ObjectLoader>()
            .init_asset_loader::<RoadLoader>()
            .add_systems(
                Update,
                wait_for_loading.run_if(in_state(GameState::ManifestsLoading)),
            );
    }

    fn finish(&self, app: &mut App) {
        // Needs to be registered in the end after all reflection registrations.
        app.init_resource::<AssetManifests>();
    }
}

fn wait_for_loading(
    mut commands: Commands,
    manifests: Res<AssetManifests>,
    asset_server: Res<AssetServer>,
) {
    let objects = manifests.objects.iter().map(|handle| handle.id().untyped());
    let roads = manifests.roads.iter().map(Into::into);
    if objects
        .chain(roads)
        .all(|handle| asset_server.is_loaded(handle))
    {
        info!("finished loading asset manifests");
        commands.set_state(GameState::Menu);
    }
}

/// Resource keep manifests loaded.
#[derive(Resource)]
struct AssetManifests {
    objects: Vec<Handle<ObjectManifest>>,
    roads: Vec<Handle<RoadManifest>>,
}

impl FromWorld for AssetManifests {
    fn from_world(world: &mut World) -> Self {
        let assets_dir =
            Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("assets");

        let mut manifests = AssetManifests {
            objects: Default::default(),
            roads: Default::default(),
        };
        let asset_server = world.resource::<AssetServer>();
        for path in WalkDir::new(&assets_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.into_path())
        {
            let Some(format) = ManifestFormat::parse(&path) else {
                continue;
            };

            let relative_path = path
                .strip_prefix(&assets_dir)
                .unwrap_or_else(|e| panic!("entries should start with {assets_dir:?}: {e}"));

            debug!("loading manifest {relative_path:?}");
            match format {
                ManifestFormat::Object => {
                    manifests.objects.push(asset_server.load(relative_path));
                }
                ManifestFormat::Road => {
                    manifests.roads.push(asset_server.load(relative_path));
                }
            }
        }

        manifests
    }
}

#[derive(Clone, Copy, EnumIter)]
enum ManifestFormat {
    Object,
    Road,
}

impl ManifestFormat {
    fn parse(path: &Path) -> Option<Self> {
        let path = path.to_str()?;

        for format in Self::iter() {
            for extension in format.extensions() {
                if path.ends_with(extension) {
                    return Some(format);
                }
            }
        }

        None
    }

    fn extensions(self) -> &'static [&'static str] {
        match self {
            ManifestFormat::Object => &["object.ron"],
            ManifestFormat::Road => &["road.ron"],
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GeneralManifest {
    pub name: String,
    pub author: String,
    pub license: String,
}

/// Maps paths inside reflected components.
#[reflect_trait]
pub(crate) trait MapPaths {
    /// Converts all paths relative to the file into absolute paths.
    fn map_paths(&mut self, dir: &Path);
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::Result;
    use bevy::{reflect::TypeRegistry, scene::ron};
    use walkdir::WalkDir;

    use super::*;
    use crate::{
        combined_scene_collider::SceneColliderConstructor,
        game_world::object::{
            door::Door,
            placing_object::{side_snap::SideSnap, wall_snap::WallSnap},
            wall_mount::WallMount,
        },
    };
    use object_manifest::ObjectManifestDeserializer;
    use road_manifest::RoadManifestDeserializer;

    #[test]
    fn deserialization() -> Result<()> {
        let mut registry = TypeRegistry::new();
        registry.register::<WallMount>();
        registry.register::<WallSnap>();
        registry.register::<SideSnap>();
        registry.register::<Door>();
        registry.register::<SceneColliderConstructor>();

        let mut objects_count = 0;
        let mut roads_count = 0;
        for path in WalkDir::new("../app/assets/base")
            .into_iter()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.into_path())
        {
            let Some(format) = ManifestFormat::parse(&path) else {
                continue;
            };

            let string = fs::read_to_string(&path)?;

            match format {
                ManifestFormat::Object => {
                    let seed = ObjectManifestDeserializer {
                        registry: &registry,
                        dir: None,
                    };
                    ron::Options::default().from_str_seed(&string, seed)?;
                    objects_count += 1;
                }
                ManifestFormat::Road => {
                    let seed = RoadManifestDeserializer { dir: None };
                    ron::Options::default().from_str_seed(&string, seed)?;
                    roads_count += 1;
                }
            }
        }

        assert!(objects_count > 0);
        assert!(roads_count > 0);

        Ok(())
    }
}
