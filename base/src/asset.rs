pub(super) mod collection;
pub mod info;
pub(super) mod material;

use std::path::Path;

use bevy::{asset::AssetPath, prelude::*};

use info::InfoPlugins;
use material::MaterialPlugin;

pub(super) struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin, InfoPlugins));
    }
}

/// Makes `asset_path` relative to `dir`.
///
/// Does nothing if the path is absolute.
pub(super) fn change_parent_dir(asset_path: &mut AssetPath, dir: &Path) {
    if asset_path.path().is_relative() {
        let new_path: AssetPath = dir.join(asset_path.path()).into();
        if let Some(label) = asset_path.take_label() {
            *asset_path = new_path.with_label(label)
        } else {
            *asset_path = new_path
        }
    }
}
