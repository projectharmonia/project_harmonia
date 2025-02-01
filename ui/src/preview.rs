use std::f32::consts::PI;

use bevy::{
    asset::RecursiveDependencyLoadState,
    pbr::wireframe::NoWireframe,
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureUsages},
        view::{NoFrustumCulling, RenderLayers},
    },
    scene,
};

use project_harmonia_base::asset::manifest::object_manifest::ObjectManifest;

pub(super) struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PreviewState>()
            .add_systems(Startup, setup)
            .add_systems(
                OnEnter(PreviewState::Inactive),
                despawn_scene.never_param_warn(),
            )
            .add_systems(OnEnter(PreviewState::Rendering), render)
            .add_systems(
                SpawnScene,
                (
                    wait_for_request
                        .before(scene::scene_spawner_system)
                        .run_if(in_state(PreviewState::Inactive)),
                    wait_for_loading
                        .after(scene::scene_spawner_system)
                        .run_if(in_state(PreviewState::LoadingAsset)),
                ),
            );
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(PreviewCamera);
    commands.spawn((
        PREVIEW_RENDER_LAYER,
        DirectionalLight::default(),
        Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn wait_for_request(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    manifests: Res<Assets<ObjectManifest>>,
    camera_entity: Single<Entity, With<PreviewCamera>>,
    previews: Query<(Entity, &Preview, Has<CalculatedClip>), Without<PreviewProcessed>>,
    actors: Query<&SceneRoot>,
) {
    // Check for `CalculatedClip` to make sure that the preview node is visible.
    if let Some((preview_entity, &preview, ..)) = previews.iter().find(|&(.., c)| !c) {
        let (translation, scene_root) = match preview {
            Preview::Actor(entity) => {
                debug!("generating preview for actor `{entity}`");

                let scene_root = actors
                    .get(entity)
                    .expect("actor for preview should have a scene handle");

                (Vec3::new(0.0, -1.67, -0.42), scene_root.clone())
            }
            Preview::Object(id) => {
                let manifest = manifests.get(id).expect("manifests should be preloaded");

                debug!("generating preview for object '{:?}'", manifest.scene);

                let scene_handle = asset_server.load(manifest.scene.clone()).into();

                (manifest.preview_translation, scene_handle)
            }
        };

        commands.entity(preview_entity).insert(PreviewProcessed);
        commands.entity(*camera_entity).with_children(|parent| {
            parent.spawn((
                PreviewTarget(preview_entity),
                scene_root,
                Transform::from_translation(translation).with_rotation(Quat::from_rotation_y(PI)), // Rotate towards camera.
            ));
        });

        commands.set_state(PreviewState::LoadingAsset);
    }
}

fn wait_for_loading(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    asset_server: Res<AssetServer>,
    preview_scene: Single<(Entity, &PreviewTarget, &SceneRoot)>,
    mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
    targets: Query<&Node>,
    children: Query<&Children>,
    meshes: Query<Entity, With<Mesh3d>>,
) {
    let (scene_entity, preview_target, scene_handle) = *preview_scene;
    match asset_server.recursive_dependency_load_state(&**scene_handle) {
        RecursiveDependencyLoadState::Loaded => {
            debug!("asset for preview was successfully loaded");

            let Ok(node) = targets.get(preview_target.0) else {
                debug!("preview target is no longer valid");
                commands.set_state(PreviewState::Inactive);
                return;
            };

            let (Val::Px(width), Val::Px(height)) = (node.width, node.height) else {
                panic!("width and height should be set in pixels");
            };

            let mut image = Image::default();
            image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
            image.resize(Extent3d {
                width: width as u32,
                height: height as u32,
                ..Default::default()
            });

            let image_handle = images.add(image);

            let mut camera = preview_cameras.single_mut();
            camera.is_active = true;
            camera.target = RenderTarget::Image(image_handle);

            for child_entity in meshes.iter_many(children.iter_descendants(scene_entity)) {
                commands.entity(child_entity).insert((
                    PREVIEW_RENDER_LAYER,
                    NoFrustumCulling,
                    NoWireframe,
                ));
            }

            commands.set_state(PreviewState::Rendering);
        }
        RecursiveDependencyLoadState::Failed(e) => {
            error!("unable to load asset: {e:#}");
            commands.set_state(PreviewState::Inactive);
        }
        RecursiveDependencyLoadState::NotLoaded | RecursiveDependencyLoadState::Loading => (),
    }
}

/// Waits one frame for components like [`NoWireframe`] to take effect.
fn render(mut commands: Commands) {
    debug!("finishing rendering");
    commands.set_state(PreviewState::Inactive);
}

fn despawn_scene(
    mut commands: Commands,
    mut preview_camera: Single<&mut Camera, With<PreviewCamera>>,
    preview_scene: Single<(Entity, &PreviewTarget)>,
    mut targets: Query<&mut ImageNode>,
) {
    preview_camera.is_active = false;

    let (entity, preview_target) = *preview_scene;
    if let Ok(mut target_handle) = targets.get_mut(**preview_target) {
        let RenderTarget::Image(image_handle) = &preview_camera.target else {
            panic!("preview camera should render only to images");
        };
        target_handle.image = image_handle.clone();
        debug!("preview is ready");
    } else {
        info!("preview target is no longer valid");
    }

    commands.entity(entity).despawn_recursive();
}

const PREVIEW_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
enum PreviewState {
    #[default]
    Inactive,
    LoadingAsset,
    Rendering,
}

/// Marker for preview camera.
#[derive(Component)]
#[require(
    Name(|| Name::new("Preview camera")),
    RenderLayers(|| PREVIEW_RENDER_LAYER),
    Transform(|| Transform::from_translation(Vec3::Y * 1000.0)), // High above the player to avoid noticing.
    Camera3d,
    Camera(|| Camera {
        is_active: false,
        order: -2,
        ..Default::default()
    }),
)]
struct PreviewCamera;

/// Specifies preview that should be generated for specific actor in the world or for an object by its manifest.
///
/// Generated image handle will be written to the image handle on this entity.
/// Preview generation happens only if UI element entity is visible.
/// Processed entities will be marked with [`PreviewProcessed`].
#[derive(Clone, Component, Copy)]
#[require(ImageNode)]
pub(crate) enum Preview {
    Actor(Entity),
    Object(AssetId<ObjectManifest>),
}

/// Marks entity with [`Preview`] as processed end excludes it from preview generation.
#[derive(Component)]
pub(super) struct PreviewProcessed;

/// Points to the entity for which the preview will be generated.
#[derive(Component, Deref, Clone, Copy)]
#[require(
    Name(|| Name::new("Preview scene")),
    SceneRoot,
)]
struct PreviewTarget(Entity);
