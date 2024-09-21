use std::{fs, path::Path};

use anyhow::{Context, Result};
use avian3d::prelude::*;
use bevy::{
    color::palettes::css::DARK_RED, pbr::wireframe::WireframeConfig, prelude::*, scene::ron,
    utils::HashMap, window::WindowMode,
};
use leafwing_input_manager::{prelude::*, user_input::InputKind};
use serde::{Deserialize, Serialize};
use strum::Display;
use vleue_navigator::prelude::*;

use super::{game_paths::GamePaths, message::error_message};

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let game_paths = app.world().resource::<GamePaths>();

        app.insert_resource(Settings::read(&game_paths.settings).unwrap_or_default())
            .add_event::<SettingsApply>()
            .init_resource::<InputMap<Action>>()
            .init_resource::<ActionState<Action>>()
            .add_systems(Startup, Self::apply)
            .add_systems(
                PostUpdate,
                (Self::write.pipe(error_message), Self::apply).run_if(on_event::<SettingsApply>()),
            );
    }
}

impl SettingsPlugin {
    fn write(settings: Res<Settings>, game_paths: Res<GamePaths>) -> Result<()> {
        settings.write(&game_paths.settings)
    }

    fn apply(
        mut commands: Commands,
        mut config_store: ResMut<GizmoConfigStore>,
        mut wireframe_config: ResMut<WireframeConfig>,
        mut input_map: ResMut<InputMap<Action>>,
        settings: Res<Settings>,
        mut windows: Query<&mut Window>,
    ) {
        info!("applying settings");

        let mut window = windows.single_mut();
        if settings.video.fullscreen {
            window.mode = WindowMode::Fullscreen;
        } else {
            window.mode = WindowMode::Windowed;
        }

        config_store.config_mut::<PhysicsGizmos>().0.enabled = settings.developer.colliders;
        wireframe_config.global = settings.developer.wireframe;
        if settings.developer.nav_mesh {
            commands.insert_resource(NavMeshesDebug(DARK_RED.into()))
        } else {
            commands.remove_resource::<NavMeshesDebug>();
        }

        input_map.clear();
        for (&action, inputs) in &settings.controls.mappings {
            input_map.insert_one_to_many(action, inputs.iter().cloned());
        }
    }
}

/// An event that applies the specified settings in the [`Settings`] resource.
#[derive(Default, Event)]
pub struct SettingsApply;

#[derive(Clone, Default, Deserialize, PartialEq, Reflect, Resource, Serialize)]
#[serde(default)]
pub struct Settings {
    pub video: VideoSettings,
    #[reflect(ignore)]
    pub controls: ControlsSettings,
    pub developer: DeveloperSettings,
}

impl Settings {
    /// Creates [`Settings`] from the application settings file.
    /// Will be initialed with defaults if the file does not exist.
    fn read(file_name: &Path) -> Result<Settings> {
        info!("reading settings from {file_name:?}");

        match fs::read_to_string(file_name) {
            Ok(content) => ron::from_str::<Settings>(&content)
                .with_context(|| format!("unable to read settings from {file_name:?}")),
            Err(_) => Ok(Settings::default()),
        }
    }

    /// Saves settings on disk under.
    ///
    /// Automatically creates all parent folders.
    fn write(&self, file_name: &Path) -> Result<()> {
        info!("writing settings to {file_name:?}");

        let content = ron::ser::to_string_pretty(&self, Default::default())
            .context("unable to serialize settings")?;

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
pub struct VideoSettings {
    /// TODO: Replace with combobox for all window modes.
    pub fullscreen: bool,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct ControlsSettings {
    pub mappings: HashMap<Action, Vec<InputKind>>,
}

impl Default for ControlsSettings {
    fn default() -> Self {
        let mappings = [
            (
                Action::CameraForward,
                vec![KeyCode::KeyW.into(), KeyCode::ArrowUp.into()],
            ),
            (
                Action::CameraBackward,
                vec![KeyCode::KeyS.into(), KeyCode::ArrowDown.into()],
            ),
            (
                Action::CameraLeft,
                vec![KeyCode::KeyA.into(), KeyCode::ArrowLeft.into()],
            ),
            (
                Action::CameraRight,
                vec![KeyCode::KeyD.into(), KeyCode::ArrowRight.into()],
            ),
            (Action::RotateCamera, vec![MouseButton::Middle.into()]),
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
pub struct DeveloperSettings {
    pub colliders: bool,
    pub paths: bool,
    pub nav_mesh: bool,
    pub wireframe: bool,
}

#[derive(
    Actionlike,
    Clone,
    Copy,
    Debug,
    Deserialize,
    Display,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Reflect,
    Serialize,
)]
pub enum Action {
    #[strum(serialize = "Camera Forward")]
    CameraForward,
    #[strum(serialize = "Camera Backward")]
    CameraBackward,
    #[strum(serialize = "Camera Left")]
    CameraLeft,
    #[strum(serialize = "Camera Right")]
    CameraRight,
    #[strum(serialize = "Rotate Camera")]
    RotateCamera,
    #[strum(serialize = "Zoom Camera")]
    ZoomCamera,
    #[strum(serialize = "Rotate Object")]
    RotateObject,
    Confirm,
    Delete,
    Cancel,
}
