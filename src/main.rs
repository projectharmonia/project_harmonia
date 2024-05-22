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

use core::{
    action::Action,
    actor::{ACTOR_HEIGHT, ACTOR_RADIUS},
    city::HALF_CITY_SIZE,
    cli::Cli,
    CorePlugins,
};
use ui::UiPlugins;

fn main() {
    App::new()
        .init_resource::<Cli>()
        .insert_resource(Msaa::Off) // Required by SSAO.
        .insert_resource(AmbientLight {
            color: Color::ANTIQUE_WHITE,
            brightness: 1000.0,
        })
        // TODO: workaround to place objects close together, remove after the next release.
        .insert_resource(NarrowPhaseConfig {
            prediction_distance: 0.0,
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
            OxidizedNavigationPlugin::<Collider>::new(
                NavMeshSettings::from_agent_and_bounds(
                    ACTOR_RADIUS,
                    ACTOR_HEIGHT,
                    HALF_CITY_SIZE,
                    0.0,
                )
                .with_walkable_radius(1),
            ),
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
