mod path_debug;

use bevy::{pbr::wireframe::WireframeConfig, prelude::*};
use bevy_xpbd_3d::prelude::*;
use oxidized_navigation::debug_draw::DrawNavMesh;

use super::settings::{Settings, SettingsApply};
use path_debug::PathDebugPlugin;

/// Propagates developer settings changes into resources.
pub(super) struct DeveloperPlugin;

impl Plugin for DeveloperPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PathDebugPlugin)
            .add_systems(
                Startup,
                (
                    Self::set_debug_collisions,
                    Self::set_wireframe,
                    Self::set_debug_paths,
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::set_debug_collisions,
                    Self::set_wireframe,
                    Self::set_debug_paths,
                )
                    .run_if(on_event::<SettingsApply>()),
            );
    }
}

impl DeveloperPlugin {
    fn set_debug_collisions(mut config_store: ResMut<GizmoConfigStore>, settings: Res<Settings>) {
        config_store.config_mut::<PhysicsGizmos>().0.enabled = settings.developer.debug_collisions;
    }

    fn set_wireframe(settings: Res<Settings>, mut wireframe_config: ResMut<WireframeConfig>) {
        wireframe_config.global = settings.developer.wireframe;
    }

    fn set_debug_paths(settings: Res<Settings>, mut draw_nav_mesh: ResMut<DrawNavMesh>) {
        draw_nav_mesh.0 = settings.developer.debug_paths;
    }
}
