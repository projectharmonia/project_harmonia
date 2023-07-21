use std::{fs, path::Path};

use anyhow::{Context, Result};
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};

use super::{action::Action, error, game_paths::GamePaths};

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let game_paths = app.world.resource::<GamePaths>();

        app.insert_resource(Settings::read(&game_paths.settings).unwrap_or_default())
            .add_event::<SettingsApply>()
            .add_systems(
                Update,
                Self::write_system
                    .pipe(error::report)
                    .run_if(on_event::<SettingsApply>()),
            );
    }
}

impl SettingsPlugin {
    fn write_system(settings: Res<Settings>, game_paths: Res<GamePaths>) -> Result<()> {
        settings.write(&game_paths.settings)
    }
}

/// An event that applies the specified settings in the [`Settings`] resource.
#[derive(Default, Event)]
pub(crate) struct SettingsApply;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Reflect, Resource, Serialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) video: VideoSettings,
    // TODO: TOML implementations have issues with [`HashSet`]:
    // https://github.com/alexcrichton/toml-rs/issues/469 and https://github.com/ordian/toml_edit/issues/319
    #[serde(skip)]
    #[reflect(ignore)]
    pub(crate) controls: ControlsSettings,
    pub(crate) developer: DeveloperSettings,
}

impl Settings {
    /// Creates [`Settings`] from the application settings file.
    /// Will be initialed with defaults if the file does not exist.
    fn read(file_name: &Path) -> Result<Settings> {
        match fs::read_to_string(file_name) {
            Ok(content) => toml::from_str::<Settings>(&content)
                .with_context(|| format!("unable to read settings from {file_name:?}")),
            Err(_) => Ok(Settings::default()),
        }
    }

    /// Saves settings on disk under.
    ///
    /// Automatically creates all parent folders.
    fn write(&self, file_name: &Path) -> Result<()> {
        let content = toml::to_string_pretty(&self).context("unable to serialize settings")?;

        let parent_folder = file_name
            .parent()
            .expect("settings filename should have a parent dir");

        fs::create_dir_all(parent_folder)
            .with_context(|| format!("unable to create {parent_folder:?}"))?;

        fs::write(file_name, content)
            .with_context(|| format!("unable to write settings to {file_name:?}"))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Reflect, Serialize)]
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct ControlsSettings {
    pub(crate) mappings: InputMap<Action>,
}

impl Default for ControlsSettings {
    fn default() -> Self {
        let mut input = InputMap::default();
        input
            .insert(KeyCode::W, Action::CameraForward)
            .insert(KeyCode::S, Action::CameraBackward)
            .insert(KeyCode::A, Action::CameraLeft)
            .insert(KeyCode::D, Action::CameraRight)
            .insert(KeyCode::Up, Action::CameraForward)
            .insert(KeyCode::Down, Action::CameraBackward)
            .insert(KeyCode::Left, Action::CameraLeft)
            .insert(KeyCode::Right, Action::CameraRight)
            .insert(MouseButton::Right, Action::RotateCamera)
            .insert(SingleAxis::mouse_wheel_y(), Action::ZoomCamera)
            .insert(MouseButton::Right, Action::RotateObject)
            .insert(MouseButton::Left, Action::Confirm)
            .insert(KeyCode::Delete, Action::Delete)
            .insert(KeyCode::Escape, Action::Cancel);

        Self { mappings: input }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Reflect, Serialize)]
#[serde(default)]
pub(crate) struct DeveloperSettings {
    pub(crate) game_inspector: bool,
    pub(crate) debug_collisions: bool,
    pub(crate) debug_paths: bool,
    pub(crate) wireframe: bool,
}
