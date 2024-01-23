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
use bevy_polyline::prelude::*;
use bevy_rapier3d::prelude::*;
use bevy_replicon::prelude::*;
use leafwing_input_manager::prelude::*;
use oxidized_navigation::{NavMeshSettings, OxidizedNavigationPlugin};

use crate::core::{action::Action, cli::Cli, CorePlugins};
use ui::UiPlugins;

fn main() {
    App::new()
        .init_resource::<Cli>()
        .insert_resource(Msaa::Off)
        .insert_resource(AmbientLight {
            color: Color::ANTIQUE_WHITE,
            brightness: 1.0,
        })
        .add_plugins((
            DefaultPlugins
                .set(LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,naga=warn,project_harmonia=debug"
                        .into(),
                    level: bevy::log::Level::DEBUG,
                })
                .set(RenderPlugin {
                    render_creation: RenderCreation::Automatic(WgpuSettings {
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..Default::default()
                    }),
                }),
            ReplicationPlugins,
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
            RapierPhysicsPlugin::<NoUserData>::default(),
            RapierDebugRenderPlugin::default(),
            // OutlinePlugin, // TODO: Re-enable when https://github.com/komadori/bevy_mod_outline/issues/27 will be fixed.
            PolylinePlugin,
            CorePlugins,
            UiPlugins,
        ))
        .run();
}
