use std::{fs::DirEntry, path::PathBuf};

use anyhow::{Context, Result};
use bevy::prelude::*;
use standard_paths::{LocationType, StandardPaths};

/// Initializes [`GamePaths`] resource.
pub(super) struct GamePathsPlugin;

impl Plugin for GamePathsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamePaths>();
    }
}

/// Paths with game files, such as settings and savegames.
pub(crate) struct GamePaths {
    pub(crate) settings: PathBuf,
    pub(crate) worlds: PathBuf,
}

impl GamePaths {
    const WORLD_EXTENSION: &'static str = "world";

    pub(crate) fn world_path(&self, world_name: &str) -> PathBuf {
        let mut path = self.worlds.join(world_name);
        path.set_extension(Self::WORLD_EXTENSION);
        path
    }

    pub(crate) fn get_world_names(&self) -> Result<Vec<String>> {
        let entries = self
            .worlds
            .read_dir()
            .with_context(|| format!("Unable to read {:?}", self.worlds))?;
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
    fn default() -> Self {
        #[cfg(test)]
        let location = LocationType::TempLocation;
        #[cfg(not(test))]
        let location = LocationType::AppConfigLocation;

        let config_dir = StandardPaths::default()
            .writable_location(location)
            .expect("Unable to locate configuration directory");

        let mut settings = config_dir.clone();
        settings.push(env!("CARGO_PKG_NAME"));
        settings.set_extension("toml");

        let mut worlds = config_dir;
        worlds.push("worlds");

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
    if extension != GamePaths::WORLD_EXTENSION {
        return None;
    }

    path.file_stem()
        .map(|path| path.to_string_lossy().to_string())
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
        File::create(
            game_paths
                .worlds
                .join(format!(".{}", GamePaths::WORLD_EXTENSION)),
        )?;
        File::create(game_paths.world_path(WORLD_NAME))?;

        let world_names = game_paths.get_world_names()?;
        assert_eq!(
            world_names,
            &[WORLD_NAME],
            "Only files with a {} extension should be in the list of names",
            GamePaths::WORLD_EXTENSION
        );

        Ok(())
    }
}
