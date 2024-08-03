use std::path::PathBuf;

use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    prelude::*,
    scene::ron,
};
use serde::Deserialize;

pub(super) struct MaterialPlugin;

impl Plugin for MaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<MaterialLoader>();
    }
}

#[derive(Default)]
struct MaterialLoader;

const MATERIAL_EXTENSION: &str = "ron";

impl AssetLoader for MaterialLoader {
    type Asset = StandardMaterial;
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

        let mut material_data: MaterialData = ron::from_str(&data)?;
        if let Some(dir) = load_context.path().parent() {
            for path in material_data.iter_paths_mut() {
                *path = dir.join(&*path);
            }
        }

        let base_color_texture = material_data
            .base_color_texture
            .map(|path| load_context.load(path));
        let metallic_roughness_texture = material_data
            .metallic_roughness_texture
            .map(|path| load_context.load(path));
        let normal_map_texture = material_data
            .normal_map_texture
            .map(|path| load_context.load(path));
        let occlusion_texture = material_data
            .occlusion_texture
            .map(|path| load_context.load(path));

        let material = StandardMaterial {
            base_color_texture,
            metallic_roughness_texture,
            normal_map_texture,
            occlusion_texture,
            perceptual_roughness: material_data.perceptual_roughness,
            reflectance: material_data.reflectance,
            ..Default::default()
        };

        Ok(material)
    }

    fn extensions(&self) -> &[&str] {
        &[MATERIAL_EXTENSION]
    }
}

#[derive(Deserialize)]
struct MaterialData {
    base_color_texture: Option<PathBuf>,
    metallic_roughness_texture: Option<PathBuf>,
    normal_map_texture: Option<PathBuf>,
    occlusion_texture: Option<PathBuf>,
    perceptual_roughness: f32,
    reflectance: f32,
}

impl MaterialData {
    /// Returns iterator over mutable references for all path that are [`Some`].
    ///
    /// Needed to make paths relative.
    fn iter_paths_mut(&mut self) -> impl Iterator<Item = &mut PathBuf> {
        [
            self.base_color_texture.as_mut(),
            self.metallic_roughness_texture.as_mut(),
            self.normal_map_texture.as_mut(),
            self.occlusion_texture.as_mut(),
        ]
        .into_iter()
        .flatten()
    }
}

impl Default for MaterialData {
    fn default() -> Self {
        let material = StandardMaterial::default();
        Self {
            base_color_texture: None,
            metallic_roughness_texture: None,
            normal_map_texture: None,
            occlusion_texture: None,
            perceptual_roughness: material.perceptual_roughness,
            reflectance: material.reflectance,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use anyhow::Result;
    use bevy::scene::ron;
    use walkdir::WalkDir;

    use super::*;

    #[test]
    fn deserialization() -> Result<()> {
        let base_dir = Path::new("../app/assets/base");

        for asset_dir in [base_dir.join("ground"), base_dir.join("walls")] {
            for entry in WalkDir::new(asset_dir)
                .into_iter()
                .filter_map(|entry| entry.ok())
            {
                if let Some(extension) = entry.path().extension() {
                    if extension == MATERIAL_EXTENSION {
                        let data = fs::read_to_string(entry.path())?;
                        ron::from_str::<MaterialData>(&data)?;
                    }
                }
            }
        }

        Ok(())
    }
}
