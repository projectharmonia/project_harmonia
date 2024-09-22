use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

#[derive(Default)]
pub struct FpsCounterPlugin;

impl Plugin for FpsCounterPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin);
        }
        app.add_systems(Update, update_text);
    }
}

fn update_text(diagnostic: Res<DiagnosticsStore>) {
    if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS) {
        println!("{:?}", fps.value());
    }
}
