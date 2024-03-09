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
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
};
use bevy_atmosphere::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::RepliconRenetPlugins;
use bevy_simple_text_input::TextInputPlugin;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::prelude::*;
use oxidized_navigation::{
    debug_draw::OxidizedNavigationDebugDrawPlugin, NavMeshSettings, OxidizedNavigationPlugin,
};

use crate::core::{action::Action, cli::Cli, CorePlugins};
use ui::UiPlugins;

fn main() {
    App::new()
        .init_resource::<Cli>()
        .insert_resource(Msaa::Off) // Required by SSAO.
        .insert_resource(AmbientLight {
            color: Color::ANTIQUE_WHITE,
            brightness: 1000.0,
        })
        .add_plugins((
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,project_harmonia=debug".into(),
                    ..Default::default()
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..Default::default()
                    }),
                    synchronous_pipeline_compilation: true,
                }),
            RepliconPlugins,
            RepliconRenetPlugins,
            WireframePlugin,
            AtmospherePlugin,
            InputManagerPlugin::<Action>::default(),
            OxidizedNavigationPlugin::<Collider>::new(NavMeshSettings {
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
            }),
            OxidizedNavigationDebugDrawPlugin,
            PhysicsPlugins::default()
                .build()
                .disable::<IntegratorPlugin>()
                .disable::<SolverPlugin>()
                .disable::<SleepingPlugin>(),
            PhysicsDebugPlugin::default(),
            TextInputPlugin,
            OutlinePlugin,
            CorePlugins,
            UiPlugins,
        ))
        .run();
}
