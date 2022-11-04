use bevy::{prelude::*, render::camera::CameraRenderGraph};
use iyes_loopless::prelude::IntoConditionalSystem;

use super::settings::{Settings, SettingsApply};

pub(super) struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::toggle_global_illumination_system.run_on_event::<SettingsApply>());
    }
}

impl VideoPlugin {
    fn toggle_global_illumination_system(
        mut commands: Commands,
        settings: Res<Settings>,
        mut cameras: Query<Entity, With<CameraRenderGraph>>,
    ) {
        for entity in &mut cameras {
            commands
                .entity(entity)
                .insert(settings.video.camera_render_graph());
        }
    }
}
