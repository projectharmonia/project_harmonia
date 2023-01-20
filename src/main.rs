#![warn(clippy::doc_markdown)]
#![allow(clippy::type_complexity)] // Do not warn about long queries
#![allow(clippy::too_many_arguments)] // Do not warn about big systems

mod core;
mod ui;

use bevy::{
    log::LogPlugin,
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::settings::{WgpuFeatures, WgpuSettings},
};
use bevy_egui::EguiPlugin;
use bevy_hikari::prelude::*;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use bevy_polyline::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin};
use bevy_scene_hook::HookPlugin;
use leafwing_input_manager::prelude::*;

use crate::core::{action::Action, cli::Cli, picking::Pickable, CorePlugins};
use ui::UiPlugins;

fn main() {
    App::new()
        .init_resource::<Cli>()
        .insert_resource(WgpuSettings {
            features: WgpuFeatures::POLYGON_MODE_LINE,
            ..Default::default()
        })
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter:
                        // Disable `bevy_render` due to wgpu error in debug mode caused by path tracing.
                        "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,bevy_render=off,dollis=debug"
                            .into(),
                    level: bevy::log::Level::DEBUG,
                })
                .set(AssetPlugin {
                    watch_for_changes: true,
                    ..Default::default()
                }),
        )
        .add_plugin(WireframePlugin)
        .add_plugin(HookPlugin)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(RenetServerPlugin::default())
        .add_plugin(RenetClientPlugin::default())
        .add_plugin(DefaultRaycastingPlugin::<Pickable>::default())
        .add_plugin(OutlinePlugin)
        .add_plugin(PolylinePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(DefaultInspectorConfigPlugin)
        .add_plugin(HikariPlugin)
        .add_plugins(CorePlugins)
        .add_plugins(UiPlugins)
        .run();
}
