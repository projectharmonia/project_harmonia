use bevy::prelude::*;

use project_harmonia_base::game_world::WorldState;

/// Spawn a dedicated camera for UI when we don't use 3D camera.
pub(super) struct Camera2dPlugin;

impl Plugin for Camera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::spawn)
            .add_systems(OnExit(WorldState::FamilyEditor), Self::enable)
            .add_systems(OnExit(WorldState::Family), Self::enable)
            .add_systems(OnExit(WorldState::City), Self::enable)
            .add_systems(OnEnter(WorldState::FamilyEditor), Self::disable)
            .add_systems(OnEnter(WorldState::Family), Self::disable)
            .add_systems(OnEnter(WorldState::City), Self::disable);
    }
}

impl Camera2dPlugin {
    fn spawn(mut commands: Commands) {
        debug!("spawning camera for menu");
        commands.spawn((
            Camera2d,
            Camera {
                // Use lower order to avoid warning when player and UI cameras
                // exists at the same time, despite we disable it.
                order: -1,
                ..Default::default()
            },
        ));
    }

    fn disable(mut ui_camera: Single<&mut Camera, With<Camera2d>>) {
        debug!("disabling camera menu");
        ui_camera.is_active = false;
    }

    fn enable(mut ui_camera: Single<&mut Camera, With<Camera2d>>) {
        debug!("disabling camera menu");
        ui_camera.is_active = true;
    }
}
