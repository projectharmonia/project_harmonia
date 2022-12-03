use bevy::{prelude::*, render::camera::CameraRenderGraph};
use iyes_loopless::prelude::IntoConditionalSystem;

use super::settings::{Settings, SettingsApply};

pub(super) struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::toggle_path_tracing_system.run_on_event::<SettingsApply>());
    }
}

impl VideoPlugin {
    fn toggle_path_tracing_system(
        settings: Res<Settings>,
        mut render_graphs: Query<&mut CameraRenderGraph>,
    ) {
        for mut render_graph in &mut render_graphs {
            render_graph.set(settings.video.render_graph_name());
        }
    }
}
