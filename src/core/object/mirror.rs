use bevy::{
    gltf::Gltf,
    prelude::*,
    reflect::TypeUuid,
    render::{
        camera::RenderTarget,
        render_resource::{AsBindGroup, Extent3d, ShaderRef, TextureUsages},
    },
};
use iyes_loopless::prelude::*;

use crate::core::{
    game_state::GameState, game_world::GameWorld, player_camera::PlayerCamera,
    unique_asset::UniqueAssetSystem,
};

pub(super) struct MirrorPlugin;

impl Plugin for MirrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(MaterialPlugin::<MirrorMaterial>::default())
            .register_type::<Mirror>()
            .add_system(
                Self::init_system
                    .run_if_resource_exists::<GameWorld>()
                    .after(UniqueAssetSystem::Init),
            )
            .add_system(Self::rotation_system.run_in_state(GameState::City))
            .add_system(Self::rotation_system.run_in_state(GameState::Family));
    }
}

impl MirrorPlugin {
    fn init_system(
        mut commands: Commands,
        mut images: ResMut<Assets<Image>>,
        mut mirror_materials: ResMut<Assets<MirrorMaterial>>,
        gltfs: Res<Assets<Gltf>>,
        mirrors: Query<(Entity, &Handle<Scene>), Added<Mirror>>,
        children: Query<&Children>,
        material_handles: Query<&Handle<StandardMaterial>>,
    ) {
        for (parent_entity, scene_handle) in &mirrors {
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
                        const RENDER_SIZE: u32 = 400;
                        let mut image = Image::default();
                        image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
                        image.resize(Extent3d {
                            width: RENDER_SIZE,
                            height: RENDER_SIZE,
                            ..Default::default()
                        });
                        let image_handle = images.add(image);
                        let material_handle = mirror_materials.add(image_handle.clone().into());

                        commands
                            .entity(child_entity)
                            .remove::<Handle<StandardMaterial>>()
                            .insert(material_handle)
                            .with_children(|parent| {
                                parent.spawn(MirrorCameraBundle::new(image_handle));
                            });
                    }
                }
            }
        }
    }

    fn rotation_system(
        player_cameras: Query<&GlobalTransform, (With<PlayerCamera>,)>,
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

#[derive(Bundle)]
struct MirrorCameraBundle {
    name: Name,
    mirror_camera: MirrorCamera,

    #[bundle]
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

#[derive(AsBindGroup, Clone, TypeUuid)]
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