use std::{
    env,
    fs::{self, DirEntry},
    path::PathBuf,
};

use anyhow::{Context, Result};
use app_dirs2::{AppDataType, AppInfo};
use bevy::prelude::*;

/// Initializes [`GamePaths`] resource.
pub(super) struct GamePathsPlugin;

impl Plugin for GamePathsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamePaths>();
    }
}

const SCENE_EXTENSION: &str = "scn";

/// Paths with game files, such as settings and savegames.
#[derive(Resource)]
pub struct GamePaths {
    pub settings: PathBuf,
    pub worlds: PathBuf,
}

impl GamePaths {
    pub fn world_path(&self, name: &str) -> PathBuf {
        let mut path = self.worlds.join(name);
        path.set_extension(SCENE_EXTENSION);
        path
    }

    pub fn get_world_names(&self) -> Result<Vec<String>> {
        let entries = self
            .worlds
            .read_dir()
            .with_context(|| format!("unable to read {:?}", self.worlds))?;
        let mut worlds = Vec::new();
        for entry in entries.filter_map(Result::ok) {
            if let Some(name) = world_name(&entry) {
                worlds.push(name);
            }
        }
        Ok(worlds)
    }
}

impl Default for GamePaths {
    /// Creates paths from the game settings directory.
    ///
    /// In tests points to a temporary folder that will be removed on destruction.
    fn default() -> Self {
        let app_info = AppInfo {
            name: env!("CARGO_PKG_NAME"),
            author: "shatur",
        };
        let config_dir = app_dirs2::app_dir(AppDataType::UserConfig, &app_info, "")
            .expect("config directory should be accessiable");
        info!("using {config_dir:?} as config directory");

        let mut settings = config_dir.clone();
        settings.push(env!("CARGO_PKG_NAME"));
        settings.set_extension("ron");

        let mut worlds = config_dir;
        worlds.push("worlds");
        fs::create_dir_all(&worlds)
            .unwrap_or_else(|e| panic!("{worlds:?} should be writable: {e}"));

        Self { settings, worlds }
    }
}

fn world_name(entry: &DirEntry) -> Option<String> {
    let file_type = entry.file_type().ok()?;
    if !file_type.is_file() {
        return None;
    }

    let path = entry.path();
    let extension = path.extension()?;
    if extension != SCENE_EXTENSION {
        return None;
    }

    path.file_stem()?.to_str().map(|stem| stem.to_string())
}
