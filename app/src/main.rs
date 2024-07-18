mod cli;

use bevy::{
    color::palettes::css::ANTIQUE_WHITE,
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

use cli::{Cli, CliPlugin};
use project_harmonia_base::{
    game_world::{
        actor::{ACTOR_HEIGHT, ACTOR_RADIUS},
        city::HALF_CITY_SIZE,
    },
    settings::Action,
    CorePlugins,
};
use project_harmonia_ui::UiPlugins;
use project_harmonia_widgets::WidgetsPlugin;

fn main() {
    App::new()
        .init_resource::<Cli>()
        .insert_resource(Msaa::Off) // Required by SSAO.
        .insert_resource(AmbientLight {
            color: ANTIQUE_WHITE.into(),
            brightness: 1000.0,
        })
        // TODO: workaround to place objects close together, remove after the next release.
        .insert_resource(NarrowPhaseConfig {
            prediction_distance: 0.0,
        })
        .add_plugins((
            DefaultPlugins.set(RenderPlugin {
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
        ))
        .add_plugins((CliPlugin, CorePlugins, WidgetsPlugin, UiPlugins))
        .run();
}
