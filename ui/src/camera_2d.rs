use bevy::prelude::*;

use project_harmonia_base::game_world::WorldState;

/// Spawn a dedicated camera for UI when we don't use 3D camera.
pub(super) struct Camera2dPlugin;

impl Plugin for Camera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::spawn)
            .add_systems(OnExit(WorldState::FamilyEditor), Self::spawn)
            .add_systems(OnExit(WorldState::Family), Self::spawn)
            .add_systems(OnExit(WorldState::City), Self::spawn)
            .add_systems(OnEnter(WorldState::FamilyEditor), Self::despawn)
            .add_systems(OnEnter(WorldState::Family), Self::despawn)
            .add_systems(OnEnter(WorldState::City), Self::despawn);
    }
}

impl Camera2dPlugin {
    fn spawn(mut commands: Commands) {
        debug!("spawning camera for menu");
        commands.spawn(Camera2dBundle::default());
    }

    fn despawn(mut commands: Commands, cameras: Query<Entity, With<Camera2d>>) {
        debug!("despawning camera for ui");
        commands.entity(cameras.single()).despawn();
    }
}
