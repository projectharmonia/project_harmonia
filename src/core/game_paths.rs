use std::{env, fs::DirEntry, path::PathBuf};

use anyhow::{Context, Result};
use bevy::prelude::*;
#[cfg(not(test))]
use standard_paths::{LocationType, StandardPaths};

/// Initializes [`GamePaths`] resource.
pub(super) struct GamePathsPlugin;

impl Plugin for GamePathsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamePaths>();
    }
}

const SCENE_EXTENSION: &str = "scn";

/// Paths with game files, such as settings and savegames.
pub(crate) struct GamePaths {
    pub(crate) settings: PathBuf,
    pub(crate) worlds: PathBuf,
    pub(crate) families: PathBuf,
}

impl GamePaths {
    pub(crate) fn world_path(&self, world_name: &str) -> PathBuf {
        let mut path = self.worlds.join(world_name);
        path.set_extension(SCENE_EXTENSION);
        path
    }

    pub(crate) fn family_path(&self, family_name: &str) -> PathBuf {
        let mut path = self.families.join(family_name);
        path.set_extension(SCENE_EXTENSION);

        let mut file_index = 0;
        while path.exists() {
            file_index += 1;
            path.set_file_name(format!("{family_name} {file_index}"));
            path.set_extension(SCENE_EXTENSION);
        }

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
        #[cfg(test)]
        let config_dir = env::temp_dir().join(
            std::iter::repeat_with(fastrand::alphanumeric)
                .take(10)
                .collect::<String>(),
        );

        #[cfg(not(test))]
        let config_dir = StandardPaths::default()
            .writable_location(LocationType::AppConfigLocation)
            .expect("app configuration directory should be found");

        let mut settings = config_dir.clone();
        settings.push(env!("CARGO_PKG_NAME"));
        settings.set_extension("toml");

        let mut worlds = config_dir.clone();
        worlds.push("worlds");

        let mut families = config_dir;
        families.push("families");

        Self {
            settings,
            worlds,
            families,
        }
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

    path.file_stem()
        .map(|path| path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};

    use super::*;

    #[test]
    fn family_path_duplication() -> Result<()> {
        let game_paths = GamePaths::default();
        const FAMILY_NAME: &str = "Test family";

        fs::create_dir_all(&game_paths.families)?;
        let family_path = game_paths.family_path(FAMILY_NAME);
        File::create(&family_path)?;
        let new_family_path = game_paths.family_path(FAMILY_NAME);

        assert_ne!(family_path, new_family_path);

        Ok(())
    }

    #[test]
    fn world_names_reading() -> Result<()> {
        let game_paths = GamePaths::default();
        const WORLD_NAME: &str = "Test world names";

        fs::create_dir_all(game_paths.worlds.join("Directory"))?;
        File::create(game_paths.worlds.join("Not a world"))?;
        File::create(game_paths.worlds.join("Not a world.txt"))?;
        File::create(game_paths.worlds.join(format!(".{}", SCENE_EXTENSION)))?;
        File::create(game_paths.world_path(WORLD_NAME))?;

        let world_names = game_paths.get_world_names()?;
        assert_eq!(world_names, &[WORLD_NAME]);

        Ok(())
    }
}
