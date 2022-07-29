use bevy::prelude::*;
use standard_paths::{LocationType, StandardPaths};
use std::{fs, path::PathBuf};

/// Initializes [`GamePaths`] resource.
pub(super) struct GamePathsPlugin;

impl Plugin for GamePathsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GamePaths>();
    }
}

/// Paths with game files, such as settings and savegames.
#[derive(Deref)]
pub(crate) struct GamePaths {
    pub(crate) settings: PathBuf,
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

        fs::create_dir_all(&config_dir)
            .unwrap_or_else(|error| panic!("Unable to create {config_dir:?}: {error}"));

        let mut settings = config_dir;
        settings.push(env!("CARGO_PKG_NAME"));
        settings.set_extension("json");

        Self { settings }
    }
}
