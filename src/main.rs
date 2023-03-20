#![warn(clippy::doc_markdown)]
#![allow(clippy::type_complexity)] // Do not warn about long queries
#![allow(clippy::too_many_arguments)] // Do not warn about big systems

mod core;
mod ui;

use bevy::{
    log::LogPlugin,
    pbr::wireframe::WireframePlugin,
    prelude::*,
    render::{
        settings::{WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_atmosphere::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::DefaultInspectorConfigPlugin;
use bevy_mod_outline::OutlinePlugin;
use bevy_polyline::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_renet::{RenetClientPlugin, RenetServerPlugin};
use bevy_scene_hook::HookPlugin;
use leafwing_input_manager::prelude::*;

use crate::core::{action::Action, cli::Cli, CorePlugins};
use ui::UiPlugins;

fn main() {
    App::new()
        .init_resource::<Cli>()
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,dollis=debug".into(),
                    level: bevy::log::Level::DEBUG,
                })
                .set(AssetPlugin {
                    watch_for_changes: true,
                    ..Default::default()
                })
                .set(RenderPlugin {
                    wgpu_settings: WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..Default::default()
                    },
                }),
        )
        .add_plugin(WireframePlugin)
        .add_plugin(AtmospherePlugin)
        .add_plugin(HookPlugin)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(RenetServerPlugin::default())
        .add_plugin(RenetClientPlugin::default())
        .add_plugin(OutlinePlugin)
        .add_plugin(PolylinePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(DefaultInspectorConfigPlugin)
        .add_plugins(CorePlugins)
        .add_plugins(UiPlugins)
        .run();
}
