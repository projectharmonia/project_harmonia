mod cli;
mod cursor_controller;

use avian3d::{prelude::*, sync::SyncConfig};
use bevy::{
    app::PluginGroupBuilder, core_pipeline::experimental::taa::TemporalAntiAliasPlugin,
    pbr::wireframe::WireframePlugin, prelude::*, render::RenderPlugin,
};
use bevy_atmosphere::prelude::*;
use bevy_enhanced_input::prelude::*;
#[cfg(feature = "inspector")]
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_mod_billboard::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;
use bevy_simple_text_input::TextInputPlugin;
use project_harmonia_base::{game_world::navigation::Obstacle, CorePlugins};
use project_harmonia_ui::UiPlugins;
use project_harmonia_widgets::WidgetsPlugin;
use vleue_navigator::prelude::*;

use cli::{Cli, CliPlugin};
use cursor_controller::CursorControllerPlugin;

struct AppPlugins;

impl PluginGroup for AppPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(CliPlugin)
            .add(CursorControllerPlugin)
    }
}

// Separate entry point for Android, which doesn't use `main.rs`.
#[bevy_main]
pub fn main() {
    let mut app = App::new();
    app.init_resource::<Cli>()
        .insert_resource(SyncConfig {
            position_to_transform: false,
            ..Default::default()
        })
        .insert_resource(Time::<Fixed>::from_hz(30.0))
        .add_plugins((
            DefaultPlugins
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..Default::default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Project Harmonia".to_string(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            TemporalAntiAliasPlugin,
            RepliconPlugins,
            RepliconRenetPlugins,
            WireframePlugin,
            AtmospherePlugin,
            EnhancedInputPlugin,
            VleueNavigatorPlugin,
            NavmeshUpdaterPlugin::<Collider, Obstacle>::default(),
            PhysicsPlugins::default()
                .build()
                .disable::<CcdPlugin>()
                .disable::<SleepingPlugin>(),
            PhysicsPickingPlugin,
            PhysicsDebugPlugin::default(),
            TextInputPlugin,
            OutlinePlugin,
            BillboardPlugin,
        ))
        .add_plugins((CorePlugins, WidgetsPlugin, UiPlugins, AppPlugins));

    #[cfg(feature = "inspector")]
    app.add_plugins(WorldInspectorPlugin::default());

    app.run();
}
