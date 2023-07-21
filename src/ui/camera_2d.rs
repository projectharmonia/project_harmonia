use bevy::prelude::*;

pub(super) struct Camera2dPlugin;

impl Plugin for Camera2dPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, Self::setup_system);
    }
}

impl Camera2dPlugin {
    fn setup_system(mut commands: Commands) {
        commands.spawn(Camera2dBundle {
            camera: Camera {
                order: -1,
                ..Default::default()
            },
            ..Default::default()
        });
    }
}
