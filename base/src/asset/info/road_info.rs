use std::path::Path;

use bevy::{
    asset::AssetPath,
    prelude::*,
    reflect::TypeRegistry,
    scene::ron::{self, error::SpannedResult},
};
use serde::{Deserialize, Serialize};

use crate::asset;

use super::{GeneralInfo, Info};

#[derive(TypePath, Serialize, Deserialize, Asset)]
pub struct RoadInfo {
    pub general: GeneralInfo,
    pub material: AssetPath<'static>,
    pub preview: AssetPath<'static>,
    pub half_width: f32,
}

impl Info for RoadInfo {
    const EXTENSION: &'static str = "road.ron";

    fn from_str(
        data: &str,
        options: ron::Options,
        _registry: &TypeRegistry,
        dir: Option<&Path>,
    ) -> SpannedResult<Self> {
        let mut info: Self = options.from_str(data)?;
        if let Some(dir) = dir {
            asset::change_parent_dir(&mut info.material, dir);
            asset::change_parent_dir(&mut info.preview, dir);
        }

        Ok(info)
    }
}
