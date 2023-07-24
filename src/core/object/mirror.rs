use bevy::{
    gltf::Gltf,
    prelude::*,
    reflect::{TypePath, TypeUuid},
    render::{
        camera::RenderTarget,
        render_resource::{AsBindGroup, Extent3d, ShaderRef, TextureUsages},
    },
    scene::{self, SceneInstance},
};

use crate::core::{game_state::GameState, game_world::WorldName, player_camera::PlayerCamera};

pub(super) struct MirrorPlugin;

impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<MirrorMaterial>::default())
            .register_type::<Mirror>()
            .add_systems(
                Update,
                (
                    Self::init_system
                        .after(scene::scene_spawner_system)
                        .run_if(resource_exists::<WorldName>()),
                    Self::rotation_system
                        .run_if(in_state(GameState::City).or_else(in_state(GameState::Family))),
                ),
            );
    }
}

impl MirrorPlugin {
    fn init_system(
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut mirror_materials: ResMut<Assets<MirrorMaterial>>,
        gltfs: Res<Assets<Gltf>>,
        scene_spawner: Res<SceneSpawner>,
        mirrors: Query<
            (Entity, &Handle<Scene>, &SceneInstance),
            (With<Mirror>, Without<MirrorReady>),
        >,
        children: Query<&Children>,
        material_handles: Query<&Handle<StandardMaterial>>,
    ) {
        for (scene_entity, scene_handle, scene_instance) in &mirrors {
            if !scene_spawner.instance_is_ready(**scene_instance) {
                continue;
            }

            let gltf = gltfs
                .iter()
                .map(|(_, gltf)| gltf)
                .find(|gltf| gltf.scenes.contains(scene_handle))
                .expect("mirror model should come from glTF");
            let Some(mirror_handle) = gltf.named_materials.get("Mirror") else {
                error!("unable to find a material named 'Mirror'");
                continue;
            };

            for child_entity in children.iter_descendants(scene_entity) {
                if let Ok(material_handle) = material_handles.get(child_entity) {
                    if material_handle == mirror_handle {
                        const RENDER_SIZE: u32 = 400;
                        let mut image = Image::default();
                        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
                        image.resize(Extent3d {
                            width: RENDER_SIZE,
                            height: RENDER_SIZE,
                            ..Default::default()
                        });
                        let image_handle = images.add(image);

                        commands
                            .entity(child_entity)
                            .remove::<Handle<StandardMaterial>>()
                            .insert(mirror_materials.add(image_handle.clone().into()))
                            .with_children(|parent| {
                                parent.spawn(MirrorCameraBundle::new(image_handle));
                            });
                    }
                }
            }

            commands.entity(scene_entity).insert(MirrorReady);
        }
    }

    fn rotation_system(
        player_cameras: Query<&GlobalTransform, With<PlayerCamera>>,
        mut mirror_cameras: Query<(&mut Transform, &GlobalTransform), With<MirrorCamera>>,
    ) {
        let player_translation = player_cameras.single().translation();
        for (mut mirror_transform, mirror_global_transform) in &mut mirror_cameras {
            let local_translation = player_translation - mirror_global_transform.translation();
            mirror_transform.look_at(
                Vec3::new(-local_translation.x, 0.0, local_translation.y),
                Vec3::Y,
            );
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub(crate) struct Mirror;

/// Marker that says that mirror materials on this entity was initialized.
#[derive(Component)]
struct MirrorReady;

#[derive(Bundle)]
struct MirrorCameraBundle {
    name: Name,
    mirror_camera: MirrorCamera,
    camera_bundle: Camera3dBundle,
}

impl MirrorCameraBundle {
    fn new(image_handle: Handle<Image>) -> Self {
        Self {
            name: "Mirror camera".into(),
            mirror_camera: MirrorCamera,
            camera_bundle: Camera3dBundle {
                camera: Camera {
                    target: RenderTarget::Image(image_handle),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }
}

#[derive(Component)]
struct MirrorCamera;

#[derive(AsBindGroup, Clone, TypeUuid, TypePath)]
#[uuid = "f425af87-cf60-493e-8bf8-dbf235e6ee46"]
struct MirrorMaterial {
    #[texture(0)]
    #[sampler(1)]
    texture: Handle<Image>,
}

impl Material for MirrorMaterial {
    fn fragment_shader() -> ShaderRef {
        "base/shaders/mirror.wgsl".into()
    }
}

impl From<Handle<Image>> for MirrorMaterial {
    fn from(texture: Handle<Image>) -> Self {
        MirrorMaterial { texture }
    }
}
