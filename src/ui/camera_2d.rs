use bevy::prelude::*;

use crate::core::game_state::GameState;

pub(super) struct Camera2dPlugin;

impl Plugin for Camera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::spawn_system)
            .add_systems(OnExit(GameState::FamilyEditor), Self::spawn_system)
            .add_systems(OnExit(GameState::Family), Self::spawn_system)
            .add_systems(OnExit(GameState::City), Self::spawn_system)
            .add_systems(OnEnter(GameState::FamilyEditor), Self::despawn_system)
            .add_systems(OnEnter(GameState::Family), Self::despawn_system)
            .add_systems(OnEnter(GameState::City), Self::despawn_system);
    }
}

impl Camera2dPlugin {
    fn spawn_system(mut commands: Commands) {
        commands.spawn(Camera2dBundle::default());
    }

    fn despawn_system(mut commands: Commands, cameras: Query<Entity, With<Camera2d>>) {
        commands.entity(cameras.single()).despawn();
    }
}
