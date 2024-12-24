use std::{fs, path::Path};

use anyhow::{Context, Result};
use avian3d::prelude::*;
use bevy::{
    color::palettes::css::DARK_RED, pbr::wireframe::WireframeConfig, prelude::*, scene::ron,
    window::WindowMode,
};
use bevy_enhanced_input::prelude::*;
use serde::{Deserialize, Serialize};
use vleue_navigator::prelude::*;

use super::{game_paths::GamePaths, message::error_message};

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        let game_paths = app.world().resource::<GamePaths>();

        app.insert_resource(Settings::read(&game_paths.settings).unwrap_or_default())
            .add_event::<SettingsApply>()
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

        wireframe_config.global = settings.developer.wireframe;
        config_store.config_mut::<PhysicsGizmos>().0.enabled = settings.developer.colliders;
        if settings.developer.nav_mesh {
            commands.insert_resource(NavMeshesDebug(DARK_RED.into()))
        } else {
            commands.remove_resource::<NavMeshesDebug>();
        }

        commands.trigger(RebuildInputContexts);
    }
}

/// An event that applies the specified settings in the [`Settings`] resource.
#[derive(Default, Event)]
pub struct SettingsApply;

#[derive(Clone, Default, Deserialize, PartialEq, Reflect, Resource, Serialize)]
#[serde(default)]
pub struct Settings {
    pub video: VideoSettings,
    pub keyboard: KeyboardSettings,
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

#[derive(Clone, Deserialize, PartialEq, Reflect, Serialize)]
#[serde(default)]
pub struct KeyboardSettings {
    pub camera_forward: Vec<KeyCode>,
    pub camera_left: Vec<KeyCode>,
    pub camera_backward: Vec<KeyCode>,
    pub camera_right: Vec<KeyCode>,
    pub rotate_left: Vec<KeyCode>,
    pub rotate_right: Vec<KeyCode>,
    pub zoom_in: Vec<KeyCode>,
    pub zoom_out: Vec<KeyCode>,
    pub delete: Vec<KeyCode>,
    pub free_placement: Vec<KeyCode>,
    pub ordinal_placement: Vec<KeyCode>,
}

impl KeyboardSettings {
    pub fn clear(&mut self) {
        self.camera_forward.clear();
        self.camera_left.clear();
        self.camera_backward.clear();
        self.camera_right.clear();
        self.rotate_left.clear();
        self.rotate_right.clear();
        self.zoom_in.clear();
        self.zoom_out.clear();
        self.delete.clear();
        self.free_placement.clear();
    }
}

impl Default for KeyboardSettings {
    fn default() -> Self {
        Self {
            camera_forward: vec![KeyCode::KeyW, KeyCode::ArrowUp],
            camera_left: vec![KeyCode::KeyA, KeyCode::ArrowLeft],
            camera_backward: vec![KeyCode::KeyS, KeyCode::ArrowDown],
            camera_right: vec![KeyCode::KeyD, KeyCode::ArrowRight],
            rotate_left: vec![KeyCode::Comma],
            rotate_right: vec![KeyCode::Period],
            zoom_in: vec![KeyCode::Equal, KeyCode::NumpadAdd],
            zoom_out: vec![KeyCode::Minus, KeyCode::NumpadSubtract],
            delete: vec![KeyCode::Delete, KeyCode::Backspace],
            free_placement: vec![KeyCode::AltLeft, KeyCode::AltRight],
            ordinal_placement: vec![KeyCode::ShiftLeft, KeyCode::ShiftRight],
        }
    }
}

#[derive(Clone, Default, Deserialize, PartialEq, Reflect, Serialize)]
#[serde(default)]
pub struct DeveloperSettings {
    pub free_camera_rotation: bool,
    pub wireframe: bool,
    pub colliders: bool,
    pub paths: bool,
    pub nav_mesh: bool,
}
