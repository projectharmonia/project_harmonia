use bevy::prelude::*;

use project_harmonia_base::core::GameState;

pub(super) struct Camera2dPlugin;

impl Plugin for Camera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::spawn)
            .add_systems(OnExit(GameState::FamilyEditor), Self::spawn)
            .add_systems(OnExit(GameState::Family), Self::spawn)
            .add_systems(OnExit(GameState::City), Self::spawn)
            .add_systems(OnEnter(GameState::FamilyEditor), Self::despawn)
            .add_systems(OnEnter(GameState::Family), Self::despawn)
            .add_systems(OnEnter(GameState::City), Self::despawn);
    }
}

impl Camera2dPlugin {
    fn spawn(mut commands: Commands) {
        commands.spawn(Camera2dBundle::default());
    }

    fn despawn(mut commands: Commands, cameras: Query<Entity, With<Camera2d>>) {
        commands.entity(cameras.single()).despawn();
    }
}
