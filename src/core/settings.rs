use std::{fs, path::Path};

use anyhow::{Context, Result};
use bevy::{prelude::*, utils::HashMap};
use leafwing_input_manager::{prelude::*, user_input::InputKind};
use serde::{Deserialize, Serialize};

use super::{action::Action, error_report, game_paths::GamePaths};

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let game_paths = app.world.resource::<GamePaths>();

        app.insert_resource(Settings::read(&game_paths.settings).unwrap_or_default())
            .add_event::<SettingsApply>()
            .add_systems(
                PostUpdate,
                Self::write_system
                    .pipe(error_report::report)
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

#[derive(Clone, Default, Deserialize, PartialEq, Reflect, Resource, Serialize)]
#[serde(default)]
pub(crate) struct Settings {
    pub(crate) video: VideoSettings,
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

#[derive(Clone, Default, Deserialize, PartialEq, Reflect, Serialize)]
#[serde(default)]
pub(crate) struct VideoSettings {
    pub(crate) perf_stats: bool,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub(crate) struct ControlsSettings {
    pub(crate) mappings: HashMap<Action, Vec<InputKind>>,
}

impl Default for ControlsSettings {
    fn default() -> Self {
        let mappings = [
            (
                Action::CameraForward,
                vec![KeyCode::W.into(), KeyCode::Up.into()],
            ),
            (
                Action::CameraBackward,
                vec![KeyCode::S.into(), KeyCode::Down.into()],
            ),
            (
                Action::CameraLeft,
                vec![KeyCode::A.into(), KeyCode::Left.into()],
            ),
            (
                Action::CameraRight,
                vec![KeyCode::D.into(), KeyCode::Right.into()],
            ),
            (Action::RotateCamera, vec![MouseButton::Right.into()]),
            (Action::ZoomCamera, vec![SingleAxis::mouse_wheel_y().into()]),
            (Action::RotateObject, vec![MouseButton::Right.into()]),
            (Action::Confirm, vec![MouseButton::Left.into()]),
            (Action::Delete, vec![KeyCode::Delete.into()]),
            (Action::Cancel, vec![KeyCode::Escape.into()]),
        ]
        .into();

        Self { mappings }
    }
}

#[derive(Clone, Default, Deserialize, PartialEq, Reflect, Serialize)]
#[serde(default)]
pub(crate) struct DeveloperSettings {
    pub(crate) game_inspector: bool,
    pub(crate) debug_collisions: bool,
    pub(crate) debug_paths: bool,
    pub(crate) wireframe: bool,
}
