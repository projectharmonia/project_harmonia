use std::{env, fs::DirEntry, path::PathBuf};

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
pub(crate) struct GamePaths {
    pub(crate) settings: PathBuf,
    pub(crate) worlds: PathBuf,
}

impl GamePaths {
    pub(crate) fn world_path(&self, world_name: &str) -> PathBuf {
        let mut path = self.worlds.join(world_name);
        path.set_extension(SCENE_EXTENSION);
        path
    }

    pub(crate) fn get_world_names(&self) -> Result<Vec<String>> {
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

        let mut settings = config_dir.clone();
        settings.push(env!("CARGO_PKG_NAME"));
        settings.set_extension("toml");

        let mut worlds = config_dir;
        worlds.push("worlds");

        Self { settings, worlds }
    }
}

/// Cleanup temporary directory used in tests.
#[cfg(test)]
impl Drop for GamePaths {
    fn drop(&mut self) {
        let config_dir = self
            .settings
            .parent()
            .expect("settings location should have a parent dir");
        std::fs::remove_dir_all(config_dir).ok();
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

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use super::*;

    #[test]
    fn world_names_reading() -> Result<()> {
        let game_paths = GamePaths::default();
        const WORLD_NAME: &str = "Test world names";

        fs::create_dir_all(game_paths.worlds.join("Directory"))?;
        File::create(game_paths.worlds.join("Not a world"))?;
        File::create(game_paths.worlds.join("Not a world.txt"))?;
        File::create(game_paths.worlds.join(format!(".{SCENE_EXTENSION}")))?;
        File::create(game_paths.world_path(WORLD_NAME))?;

        let world_names = game_paths.get_world_names()?;
        assert_eq!(world_names, &[WORLD_NAME]);

        Ok(())
    }
}
