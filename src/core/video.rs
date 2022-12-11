use bevy::{prelude::*, render::camera::CameraRenderGraph};
use bevy_hikari::prelude::*;
use iyes_loopless::prelude::IntoConditionalSystem;

use super::settings::{Settings, SettingsApply};

pub(super) struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(Self::init_path_tracing_system)
            .add_system(Self::toggle_path_tracing_system.run_on_event::<SettingsApply>());
    }
}

impl VideoPlugin {
    fn init_path_tracing_system(
        settings: Res<Settings>,
        mut hikari_settings: ResMut<HikariUniversalSettings>,
    ) {
        hikari_settings.build_mesh_acceleration_structure = settings.video.path_tracing;
        hikari_settings.build_instance_acceleration_structure = settings.video.path_tracing;
    }

    fn toggle_path_tracing_system(
        settings: Res<Settings>,
        mut hikari_settings: ResMut<HikariUniversalSettings>,
        mut render_graphs: Query<&mut CameraRenderGraph>,
    ) {
        for mut render_graph in &mut render_graphs {
            render_graph.set(settings.video.render_graph_name());
            hikari_settings.build_mesh_acceleration_structure = settings.video.path_tracing;
            hikari_settings.build_instance_acceleration_structure = settings.video.path_tracing;
        }
    }
}
