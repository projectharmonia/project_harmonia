// Conditionally enable nightly-only `no_coverage` attribute. Will be removed after stabilization, should happen soon: https://github.com/rust-lang/rust/issues/84605
#![cfg_attr(coverage, feature(no_coverage))]
#![warn(clippy::doc_markdown)]
#![allow(clippy::type_complexity)] // Do not warn about long queries
#![allow(clippy::too_many_arguments)] // Do not warn about big systems

mod core;
mod ui;

use bevy::{asset::AssetServerSettings, log::LogSettings, prelude::*};
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_raycast::DefaultRaycastingPlugin;
use bevy_rapier3d::prelude::*;
use bevy_scene_hook::HookPlugin;
use leafwing_input_manager::prelude::*;

use crate::core::{cli::Cli, control_action::ControlAction, object::ObjectPath, CorePlugins};
use ui::{ui_action::UiAction, UiPlugins};

struct DollisPlugins;

impl PluginGroup for DollisPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        DefaultPlugins.build(group);

        group
            .add(HookPlugin)
            .add(InputManagerPlugin::<UiAction>::default())
            .add(InputManagerPlugin::<ControlAction>::default())
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            .add(RapierDebugRenderPlugin::default())
            .add(DefaultRaycastingPlugin::<ObjectPath>::default())
            .add(OutlinePlugin)
            .add(EguiPlugin)
            .add(WorldInspectorPlugin::new());

        CorePlugins.build(group);
        UiPlugins.build(group);
    }
}

fn main() {
    App::new()
        .init_resource::<Cli>()
        .insert_resource(LogSettings {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,dollis=debug".into(),
            level: bevy::log::Level::DEBUG,
        })
        .insert_resource(AssetServerSettings {
            watch_for_changes: true,
            ..Default::default()
        })
        .add_plugins(DollisPlugins)
        .run();
}
