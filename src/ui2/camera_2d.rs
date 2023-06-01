use bevy::prelude::*;

pub(super) struct Camera2dPlugin;

impl Plugin for Camera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::setup_system);
    }
}

impl Camera2dPlugin {
    fn setup_system(mut commands: Commands) {
        commands.spawn(Camera2dBundle::default());
    }
}
