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
use bevy_mod_outline::OutlinePlugin;
use bevy_polyline::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use bevy_scene_hook::HookPlugin;
use leafwing_input_manager::prelude::*;
use oxidized_navigation::{NavMeshSettings, OxidizedNavigationPlugin};

use crate::core::{action::Action, cli::Cli, CorePlugins};
use ui::UiPlugins;

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 0.3,
        })
        .init_resource::<Cli>()
        .add_plugins(
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,lifescape=debug".into(),
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
        .add_plugins(ReplicationPlugins)
        .add_plugin(WireframePlugin)
        .add_plugin(AtmospherePlugin)
        .add_plugin(HookPlugin)
        .add_plugin(InputManagerPlugin::<Action>::default())
        .add_plugin(OxidizedNavigationPlugin {
            settings: NavMeshSettings {
                cell_width: 0.25,
                cell_height: 0.1,
                tile_width: 100,
                world_half_extents: 250.0,
                world_bottom_bound: -100.0,
                max_traversable_slope_radians: (40.0_f32 - 0.1).to_radians(),
                walkable_height: 20,
                walkable_radius: 1,
                step_height: 3,
                min_region_area: 100,
                merge_region_area: 500,
                max_contour_simplification_error: 1.1,
                max_edge_length: 80,
                max_tile_generation_tasks: None,
            },
        })
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_plugin(OutlinePlugin)
        .add_plugin(PolylinePlugin)
        .add_plugins(CorePlugins)
        .add_plugins(UiPlugins)
        .run();
}
