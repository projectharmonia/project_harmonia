use bevy::prelude::*;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use super::game_paths::GamePaths;

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let game_paths = app.world.resource::<GamePaths>();

        app.insert_resource(Settings::read(&game_paths.settings))
            .add_event::<SettingsApplied>()
            .add_system(Self::write_system.run_on_event::<SettingsApplied>());
    }
}

impl SettingsPlugin {
    fn write_system(settings: Res<Settings>, game_paths: Res<GamePaths>) {
        settings.write(&game_paths.settings);
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
    #[must_use]
    fn read(file_name: &Path) -> Settings {
        match fs::read_to_string(file_name) {
            Ok(content) => {
                serde_json::from_str::<Settings>(&content).expect("Unable to parse setting file")
            }
            Err(_) => Settings::default(),
        }
    }

    /// Serialize [`Settings`] on disk under [`self.file_path`].
    fn write(&self, file_name: &Path) {
        let content = serde_json::to_string_pretty(&self).expect("Unable to serialize settings");
        fs::write(file_name, content).expect("Unable to write settings");
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
    fn read_write() {
        let mut app = App::new();
        app.init_resource::<GamePaths>().add_plugin(SettingsPlugin);

        let game_paths = app.world.resource::<GamePaths>();
        assert!(
            !game_paths.exists(),
            "Settings file shouldn't be created on startup"
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

        let loaded_settings = Settings::read(&game_paths.settings);
        let settings = app.world.resource::<Settings>();
        assert_eq!(
            settings.video, loaded_settings.video,
            "Loaded settings should be equal to saved"
        );

        fs::remove_file(&game_paths.settings).expect("Saved file should be removed after the test");
    }
}
