// Conditionally enable nightly-only `no_coverage` attribute. Will be removed after stabilization, should happen soon: https://github.com/rust-lang/rust/issues/84605
#![cfg_attr(coverage, feature(no_coverage))]
#![warn(clippy::doc_markdown)]

mod core;
mod ui;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use bevy_inspector_egui::prelude::*;
use bevy_rapier3d::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::core::{control_action::ControlAction, CorePlugins};
use ui::{ui_action::UiAction, UiPlugins};

struct DollisPlugins;

impl PluginGroup for DollisPlugins {
    fn build(&mut self, group: &mut bevy::app::PluginGroupBuilder) {
        DefaultPlugins.build(group);

        group
            .add(InputManagerPlugin::<UiAction>::default())
            .add(RapierDebugRenderPlugin::default())
            .add(InputManagerPlugin::<ControlAction>::default())
            .add(RapierPhysicsPlugin::<NoUserData>::default())
            .add(EguiPlugin)
            .add(WorldInspectorPlugin::new());

        CorePlugins.build(group);
        UiPlugins.build(group);
    }
}

fn main() {
    App::new().add_plugins(DollisPlugins).run();
}
