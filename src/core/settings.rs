use anyhow::{Context, Result};
use bevy::prelude::*;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use super::{errors::log_err_system, game_paths::GamePaths};

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let game_paths = app.world.resource::<GamePaths>();

        app.insert_resource(Settings::read(&game_paths.settings).unwrap_or_default())
            .add_event::<SettingsApplied>()
            .add_system(
                Self::write_system
                    .chain(log_err_system)
                    .run_on_event::<SettingsApplied>(),
            );
    }
}

impl SettingsPlugin {
    fn write_system(settings: Res<Settings>, game_paths: Res<GamePaths>) -> Result<()> {
        settings.write(&game_paths.settings)
    }
}

/// An event that applies the specified settings in the [`Settings`] resource.
pub(crate) struct SettingsApplied;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) video: VideoSettings,
    pub(crate) developer: DeveloperSettings,
}

impl Settings {
    /// Creates [`Settings`] from the application settings file.
    /// Will be initialed with defaults if the file does not exist.
    fn read(file_name: &Path) -> Result<Settings> {
        match fs::read_to_string(file_name) {
            Ok(content) => serde_json::from_str::<Settings>(&content)
                .with_context(|| format!("Unable to read settings from {file_name:?}")),
            Err(_) => Ok(Settings::default()),
        }
    }

    /// Saves settings on disk under.
    ///
    /// Automatically creates all parent folders.
    fn write(&self, file_name: &Path) -> Result<()> {
        let content =
            serde_json::to_string_pretty(&self).context("Unable to serialize settings")?;

        if let Some(config_dir) = file_name.parent() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("Unable to create {config_dir:?}"))?;
        }

        fs::write(file_name, content)
            .with_context(|| format!("Unable to write settings to {file_name:?}"))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct VideoSettings {
    pub(crate) msaa: u32,
    pub(crate) perf_stats: bool,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            msaa: 1,
            perf_stats: false,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct DeveloperSettings {
    pub(crate) world_inspector: bool,
    pub(crate) debug_collisions: bool,
}

#[cfg(test)]
mod tests {
    use bevy::ecs::event::Events;

    use super::*;

    #[test]
    fn read_write() -> Result<()> {
        let mut app = App::new();
        app.init_resource::<GamePaths>().add_plugin(SettingsPlugin);

        let game_paths = app.world.resource::<GamePaths>();
        assert!(
            !game_paths.settings.exists(),
            "Settings file {:?} shouldn't be created on startup",
            game_paths.settings
        );

        let mut settings = app.world.resource_mut::<Settings>();
        assert_eq!(
            *settings,
            Settings::default(),
            "Settings should be defaulted if settings file does not exist"
        );

        // Modify settings
        settings.video.msaa += 1;

        let mut apply_events = app.world.resource_mut::<Events<SettingsApplied>>();
        apply_events.send(SettingsApplied);

        app.update();

        let game_paths = app.world.resource::<GamePaths>();
        assert!(
            game_paths.settings.exists(),
            "Configuration file should be created on apply event"
        );

        let loaded_settings = Settings::read(&game_paths.settings)?;
        let settings = app.world.resource::<Settings>();
        assert_eq!(
            *settings, loaded_settings,
            "Loaded settings should be equal to saved"
        );

        fs::remove_file(&game_paths.settings).context("Unable to remove saved file after the test")
    }
}
