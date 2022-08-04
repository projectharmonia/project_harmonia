use bevy::prelude::*;
use standard_paths::{LocationType, StandardPaths};
use std::path::PathBuf;

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
    pub(super) fn world_path(&self, world_name: &str) -> PathBuf {
        let mut path = self.worlds.clone();
        path.push(world_name);
        path.set_extension("world");
        path
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
        settings.set_extension("json");

        let mut worlds = config_dir;
        worlds.push("worlds");

        Self { settings, worlds }
    }
}
