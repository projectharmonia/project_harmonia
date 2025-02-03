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

use super::{error_message::error_message, game_paths::GamePaths};

pub(super) struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(apply.pipe(error_message))
            .add_systems(Startup, load);
    }
}

fn load(
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
    mut wireframe_config: ResMut<WireframeConfig>,
    game_paths: Res<GamePaths>,
    mut window: Single<&mut Window>,
) {
    info!("loading settings");

    let settings = Settings::read(&game_paths.settings).unwrap_or_default();

    apply_settings(
        &mut commands,
        &mut config_store,
        &mut wireframe_config,
        &mut window,
        &settings,
    );

    commands.insert_resource(settings);
}

fn apply(
    _trigger: Trigger<SettingsApply>,
    mut commands: Commands,
    mut config_store: ResMut<GizmoConfigStore>,
    mut wireframe_config: ResMut<WireframeConfig>,
    settings: Res<Settings>,
    game_paths: Res<GamePaths>,
    mut window: Single<&mut Window>,
) -> Result<()> {
    info!("applying settings");

    apply_settings(
        &mut commands,
        &mut config_store,
        &mut wireframe_config,
        &mut window,
        &settings,
    );

    settings.write(&game_paths.settings)
}

fn apply_settings(
    commands: &mut Commands,
    config_store: &mut GizmoConfigStore,
    wireframe_config: &mut WireframeConfig,
    window: &mut Window,
    settings: &Settings,
) {
    if settings.video.fullscreen {
        window.mode = WindowMode::Fullscreen(MonitorSelection::Current);
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

/// An event that applies the specified settings in the [`Settings`] resource.
#[derive(Event)]
pub struct SettingsApply;

#[derive(Clone, Default, Deserialize, Reflect, Resource, Serialize)]
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

#[derive(Clone, Default, Deserialize, Reflect, Serialize)]
#[serde(default)]
pub struct VideoSettings {
    /// TODO: Replace with combobox for all window modes.
    pub fullscreen: bool,
}

#[derive(Clone, Deserialize, Reflect, Serialize)]
#[serde(default)]
pub struct KeyboardSettings {
    pub camera_forward: Vec<Input>,
    pub camera_left: Vec<Input>,
    pub camera_backward: Vec<Input>,
    pub camera_right: Vec<Input>,
    pub rotate_left: Vec<Input>,
    pub rotate_right: Vec<Input>,
    pub zoom_in: Vec<Input>,
    pub zoom_out: Vec<Input>,
    pub delete: Vec<Input>,
    pub free_placement: Vec<Input>,
    pub ordinal_placement: Vec<Input>,
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
            camera_forward: vec![KeyCode::KeyW.into(), KeyCode::ArrowUp.into()],
            camera_left: vec![KeyCode::KeyA.into(), KeyCode::ArrowLeft.into()],
            camera_backward: vec![KeyCode::KeyS.into(), KeyCode::ArrowDown.into()],
            camera_right: vec![KeyCode::KeyD.into(), KeyCode::ArrowRight.into()],
            rotate_left: vec![KeyCode::Comma.into()],
            rotate_right: vec![KeyCode::Period.into()],
            zoom_in: vec![KeyCode::Equal.into(), KeyCode::NumpadAdd.into()],
            zoom_out: vec![KeyCode::Minus.into(), KeyCode::NumpadSubtract.into()],
            delete: vec![KeyCode::Delete.into(), KeyCode::Backspace.into()],
            free_placement: vec![KeyCode::AltLeft.into(), KeyCode::AltRight.into()],
            ordinal_placement: vec![KeyCode::ShiftLeft.into(), KeyCode::ShiftRight.into()],
        }
    }
}

#[derive(Clone, Default, Deserialize, Reflect, Serialize)]
#[serde(default)]
pub struct DeveloperSettings {
    pub free_camera_rotation: bool,
    pub wireframe: bool,
    pub colliders: bool,
    pub paths: bool,
    pub nav_mesh: bool,
}
