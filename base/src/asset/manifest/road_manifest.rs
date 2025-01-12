use std::path::Path;

use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, AsyncReadExt, LoadContext},
    prelude::*,
    scene::ron,
};
use serde::{de::DeserializeSeed, Deserialize, Deserializer, Serialize};

use super::{GeneralManifest, ManifestFormat, MapPaths};
use crate::asset;

#[derive(Default)]
pub(super) struct RoadLoader;

impl AssetLoader for RoadLoader {
    type Asset = RoadManifest;
    type Settings = ();
    type Error = anyhow::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut string = String::new();
        reader.read_to_string(&mut string).await?;

        let dir = load_context.path().parent();
        let seed = RoadManifestDeserializer { dir };

        let manifest = ron::Options::default().from_str_seed(&string, seed)?;

        Ok(manifest)
    }

    fn extensions(&self) -> &[&str] {
        ManifestFormat::Road.extensions()
    }
}

#[derive(TypePath, Serialize, Deserialize, Asset)]
pub struct RoadManifest {
    pub general: GeneralManifest,
    pub material: AssetPath<'static>,
    pub preview: AssetPath<'static>,
    pub half_width: f32,
}

impl MapPaths for RoadManifest {
    fn map_paths(&mut self, dir: &Path) {
        asset::change_parent_dir(&mut self.material, dir);
        asset::change_parent_dir(&mut self.preview, dir);
    }
}

pub(super) struct RoadManifestDeserializer<'a> {
    pub(super) dir: Option<&'a Path>,
}

impl<'de> DeserializeSeed<'de> for RoadManifestDeserializer<'_> {
    type Value = RoadManifest;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        RoadManifest::deserialize(deserializer).map(|mut manifest| {
            if let Some(dir) = self.dir {
                manifest.map_paths(dir);
            }
            manifest
        })
    }
}
