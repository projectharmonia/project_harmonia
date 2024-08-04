use bevy::{
    asset::{io::Reader, AssetLoader, AssetPath, AsyncReadExt, LoadContext},
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
            for asset_path in [
                material_data.base_color_texture.as_mut(),
                material_data.metallic_roughness_texture.as_mut(),
                material_data.normal_map_texture.as_mut(),
                material_data.occlusion_texture.as_mut(),
            ]
            .into_iter()
            .flatten()
            {
                super::change_parent_dir(asset_path, dir);
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
    base_color_texture: Option<AssetPath<'static>>,
    metallic_roughness_texture: Option<AssetPath<'static>>,
    normal_map_texture: Option<AssetPath<'static>>,
    occlusion_texture: Option<AssetPath<'static>>,
    perceptual_roughness: f32,
    reflectance: f32,
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

    use anyhow::{Context, Result};
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
                        ron::from_str::<MaterialData>(&data)
                            .with_context(|| format!("unable to parse {:?}", entry.path()))?;
                    }
                }
            }
        }

        Ok(())
    }
}
