use bevy::{gltf::Gltf, prelude::*, scene::SceneInstance};
use bevy_basic_portals::{AsPortalDestination, CreatePortal};

use crate::core::{game_world::WorldState, player_camera::PlayerCamera};

pub(super) struct MirrorPlugin;

impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Mirror>()
            .add_system(Self::init_system.in_set(OnUpdate(WorldState::InWorld)));
    }
}

impl MirrorPlugin {
    fn init_system(
        mut commands: Commands,
        gltfs: Res<Assets<Gltf>>,
        scene_spawner: Res<SceneSpawner>,
        mirrors: Query<
            (Entity, &Handle<Scene>, &SceneInstance),
            (With<Mirror>, Without<MirrorReady>),
        >,
        children: Query<&Children>,
        player_cameras: Query<Entity, With<PlayerCamera>>,
        material_handles: Query<&Handle<StandardMaterial>>,
    ) {
        for (parent_entity, scene_handle, scene_instance) in &mirrors {
            if scene_spawner.instance_is_ready(**scene_instance) {
                let gltf = gltfs
                    .iter()
                    .map(|(_, gltf)| gltf)
                    .find(|gltf| gltf.scenes.contains(scene_handle))
                    .expect("mirror model should come from glTF");

                let Some(mirror_handle) = gltf.named_materials.get("Mirror") else {
                    error!("unable to find a material named 'Mirror'");
                    continue;
                };

                for child_entity in children.iter_descendants(parent_entity) {
                    if let Ok(material_handle) = material_handles.get(child_entity) {
                        if material_handle == mirror_handle {
                            commands.entity(child_entity).insert(CreatePortal {
                                destination: AsPortalDestination::CreateMirror,
                                main_camera: Some(player_cameras.single()),
                                ..Default::default()
                            });
                        }
                    }
                }

                commands.entity(parent_entity).insert(MirrorReady);
            }
        }
    }
}

/// Indicates that the entity has children with mirror materials.
///
/// After scene loading all materials named `Mirror` on this entity will be replaced with portals that act like mirrors.
/// After replacement [`MirrorReady`] will be inserted.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct Mirror;

/// Marker that says that mirror materials on this entity was initialized.
#[derive(Component)]
struct MirrorReady;
