pub mod object_info;

use std::{
    env,
    marker::PhantomData,
    path::{Path, PathBuf},
};

use anyhow::Result;
use bevy::{
    app::PluginGroupBuilder,
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    reflect::{TypeRegistry, TypeRegistryArc},
    scene::ron::{self, error::SpannedResult},
};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use object_info::ObjectInfo;

pub(super) struct InfoPlugins;

impl PluginGroup for InfoPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(InfoPlugin::<ObjectInfo>::default())
    }
}

struct InfoPlugin<A>(PhantomData<A>);

impl<A> Default for InfoPlugin<A> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Asset + Info> Plugin for InfoPlugin<T> {
    fn build(&self, app: &mut App) {
        app.init_asset::<T>()
            .init_asset_loader::<InfoLoader<T>>()
            .init_resource::<InfoHandles<T>>();
    }
}

pub struct InfoLoader<A> {
    registry: TypeRegistryArc,
    marker: PhantomData<A>,
}

impl<A> FromWorld for InfoLoader<A> {
    fn from_world(world: &mut World) -> Self {
        Self {
            registry: world.resource::<AppTypeRegistry>().0.clone(),
            marker: PhantomData,
        }
    }
}

impl<A: Asset + Info> AssetLoader for InfoLoader<A> {
    type Asset = A;
    type Settings = ();
    type Error = anyhow::Error;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut data = String::new();
        reader.read_to_string(&mut data).await?;

        let mut info = A::from_str(&data, ron::Options::default(), &self.registry.read())?;
        if let Some(dir) = load_context.path().parent() {
            for path in info.iter_paths_mut() {
                *path = dir.join(&*path);
            }
        }

        Ok(info)
    }

    fn extensions(&self) -> &[&str] {
        &[A::EXTENSION]
    }
}

/// Preloads and stores info handles.
#[derive(Resource)]
#[allow(dead_code)]
struct InfoHandles<A: Asset>(Vec<Handle<A>>);

impl<A: Asset + Info> FromWorld for InfoHandles<A> {
    fn from_world(world: &mut World) -> Self {
        let assets_dir =
            Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap_or_default()).join("assets");

        let mut handles = Vec::new();
        let asset_server = world.resource::<AssetServer>();
        for entry in WalkDir::new(&assets_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            // Use `ends_with` because extension consists of 2 dots.
            if entry
                .path()
                .to_str()
                .is_some_and(|path| path.ends_with(A::EXTENSION))
            {
                let path = entry
                    .path()
                    .strip_prefix(&assets_dir)
                    .unwrap_or_else(|e| panic!("entries should start with {assets_dir:?}: {e}"));

                debug!("loading info for {path:?}");
                handles.push(asset_server.load(path.to_path_buf()));
            }
        }

        Self(handles)
    }
}

trait Info: Sized {
    /// Directory from which files should be preloaded.
    const EXTENSION: &'static str;

    fn from_str(data: &str, options: ron::Options, registry: &TypeRegistry) -> SpannedResult<Self>;

    /// Returns iterator over mutable references of all paths.
    ///
    /// Needed to convert from paths relative to the file into absolute paths.
    fn iter_paths_mut(&mut self) -> impl Iterator<Item = &mut PathBuf>;
}

#[derive(Serialize, Deserialize)]
pub struct GeneralInfo {
    pub name: String,
    pub author: String,
    pub license: String,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use anyhow::{Context, Result};
    use bevy::scene::ron;
    use walkdir::WalkDir;

    use super::*;
    use crate::game_world::object::{
        door::Door,
        placing_object::{side_snap::SideSnap, wall_snap::WallSnap},
        wall_mount::WallMount,
    };

    #[test]
    fn deserialization() -> Result<()> {
        let mut registry = TypeRegistry::new();
        registry.register::<Vec2>();
        registry.register::<Vec<Vec2>>();
        registry.register::<WallMount>();
        registry.register::<WallSnap>();
        registry.register::<SideSnap>();
        registry.register::<Door>();

        deserialize::<ObjectInfo>(&registry)?;

        Ok(())
    }

    fn deserialize<A: Info>(registry: &TypeRegistry) -> Result<()> {
        for entry in WalkDir::new("../app/assets/base")
            .into_iter()
            .filter_map(|entry| entry.ok())
        {
            // Use `ends_with` because extension consists of 2 dots.
            if entry
                .path()
                .to_str()
                .is_some_and(|path| path.ends_with(A::EXTENSION))
            {
                let data = fs::read_to_string(entry.path())?;
                A::from_str(&data, ron::Options::default(), registry)
                    .with_context(|| format!("unable to parse {:?}", entry.path()))?;
            }
        }

        Ok(())
    }
}
