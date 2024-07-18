use std::f32::consts::PI;

use bevy::{
    asset::{LoadState, RecursiveDependencyLoadState},
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureUsages},
        view::{NoFrustumCulling, RenderLayers},
    },
};

use project_harmonia_base::asset::metadata::{self, object_metadata::ObjectMetadata};

pub(super) struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<PreviewState>()
            .add_systems(Startup, Self::setup)
            .add_systems(OnEnter(PreviewState::Inactive), Self::despawn_scene)
            .add_systems(
                Update,
                (
                    Self::wait_for_request.run_if(in_state(PreviewState::Inactive)),
                    Self::wait_for_loading.run_if(in_state(PreviewState::LoadingAsset)),
                    Self::render_preview.run_if(in_state(PreviewState::Rendering)),
                ),
            );
    }
}

impl PreviewPlugin {
    fn setup(mut commands: Commands) {
        commands.spawn(PreviewCameraBundle::default());
        commands.spawn((
            PREVIEW_RENDER_LAYER,
            DirectionalLightBundle {
                transform: Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                ..Default::default()
            },
        ));
    }

    fn wait_for_request(
        mut commands: Commands,
        mut preview_state: ResMut<NextState<PreviewState>>,
        asset_server: Res<AssetServer>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        previews: Query<(Entity, &Preview, Has<CalculatedClip>), Without<PreviewProcessed>>,
        actors: Query<&Handle<Scene>>,
        preview_cameras: Query<Entity, With<PreviewCamera>>,
    ) {
        if let Some((preview_entity, &preview, ..)) = previews.iter().find(|&(.., c)| !c) {
            let (translation, scene_handle) = match preview {
                Preview::Actor(entity) => {
                    debug!("generating preview for actor `{entity:?}`");

                    let scene_handle = actors
                        .get(entity)
                        .expect("actor for preview should have a scene handle");

                    (Vec3::new(0.0, -1.67, -0.42), scene_handle.clone())
                }
                Preview::Object(id) => {
                    let metadata_path = asset_server
                        .get_path(id)
                        .expect("metadata should always come from file");
                    let scene_path = metadata::gltf_asset(&metadata_path, "Scene0");
                    debug!("generating preview for object '{scene_path:?}'");

                    let metadata = object_metadata.get(id).unwrap();
                    let scene_handle = asset_server.load(scene_path);

                    (metadata.general.preview_translation, scene_handle)
                }
            };

            commands.entity(preview_entity).insert(PreviewProcessed);
            commands
                .entity(preview_cameras.single())
                .with_children(|parent| {
                    parent.spawn(PreviewSceneBundle::new(
                        translation,
                        scene_handle,
                        preview_entity,
                    ));
                });

            preview_state.set(PreviewState::LoadingAsset);
        }
    }

    fn wait_for_loading(
        mut preview_state: ResMut<NextState<PreviewState>>,
        mut images: ResMut<Assets<Image>>,
        asset_server: Res<AssetServer>,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_scenes: Query<(&PreviewTarget, &Handle<Scene>)>,
        targets: Query<&Style>,
    ) {
        let (preview_target, scene_handle) = preview_scenes.single();
        let asset_state = asset_server.load_state(scene_handle);
        let deps_state = asset_server.recursive_dependency_load_state(scene_handle);
        if asset_state == LoadState::Loaded && deps_state == RecursiveDependencyLoadState::Loaded {
            debug!("asset for preview was sucessfully loaded");

            let Ok(style) = targets.get(preview_target.0) else {
                debug!("preview target is no longer valid");
                preview_state.set(PreviewState::Inactive);
                return;
            };

            let (Val::Px(width), Val::Px(height)) = (style.width, style.height) else {
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
            camera.target = RenderTarget::Image(image_handle.clone());

            preview_state.set(PreviewState::Rendering);
        } else if matches!(asset_state, LoadState::Failed(_))
            || deps_state == RecursiveDependencyLoadState::Failed
        {
            error!("unable to load asset");
            preview_state.set(PreviewState::Inactive);
        }
    }

    fn render_preview(
        mut commands: Commands,
        mut preview_state: ResMut<NextState<PreviewState>>,
        preview_scenes: Query<Entity, With<PreviewTarget>>,
        chidlren: Query<&Children>,
        meshes: Query<Entity, With<Handle<Mesh>>>,
    ) {
        debug!("waiting for rendering");
        let scene_entity = preview_scenes.single();
        for child_entity in meshes.iter_many(chidlren.iter_descendants(scene_entity)) {
            commands
                .entity(child_entity)
                .insert((PREVIEW_RENDER_LAYER, NoFrustumCulling));
        }

        preview_state.set(PreviewState::Inactive);
    }

    fn despawn_scene(
        mut commands: Commands,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_scenes: Query<(Entity, &PreviewTarget)>,
        mut targets: Query<&mut Handle<Image>>,
    ) {
        let Ok(mut preview_camera) = preview_cameras.get_single_mut() else {
            return;
        };
        preview_camera.is_active = false;

        if let Ok((entity, preview_target)) = preview_scenes.get_single() {
            if let Ok(mut target_handle) = targets.get_mut(preview_target.0) {
                let RenderTarget::Image(image_handle) = &preview_camera.target else {
                    panic!("preview camera should render only to images");
                };
                *target_handle = image_handle.clone();
                debug!("preview is ready");
            } else {
                info!("preview target is no longer valid");
            }

            commands.entity(entity).despawn_recursive();
        }
    }
}

const PREVIEW_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Bundle)]
struct PreviewCameraBundle {
    name: Name,
    preview_camera: PreviewCamera,
    render_layer: RenderLayers,
    camera_bundle: Camera3dBundle,
    visibility_bundle: VisibilityBundle,
}

impl Default for PreviewCameraBundle {
    fn default() -> Self {
        Self {
            name: "Preview camera".into(),
            preview_camera: PreviewCamera,
            render_layer: PREVIEW_RENDER_LAYER,
            camera_bundle: Camera3dBundle {
                transform: Transform::from_translation(Vec3::Y * 1000.0), // High above the player to avoid noticing.
                camera: Camera {
                    is_active: false,
                    order: -2,
                    ..Default::default()
                },
                ..Default::default()
            },
            // Preview scenes will be spawned as children so this component is necessary in order to have scenes visible.
            visibility_bundle: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
enum PreviewState {
    #[default]
    Inactive,
    LoadingAsset,
    Rendering,
}

/// Marker for preview camera.
#[derive(Component)]
struct PreviewCamera;

/// Specifies preview that should be generated for specific actor in the world or for an object by its metadata.
///
/// Generated image handle will be written to the image handle on this entity.
/// Preview generation happens only if UI element entity is visible.
/// Processed entities will be marked with [`PreviewProcessed`].
#[derive(Clone, Component, Copy)]
pub(crate) enum Preview {
    Actor(Entity),
    Object(AssetId<ObjectMetadata>),
}

/// Marks entity with [`Preview`] as processed end excludes it from preview generation.
#[derive(Component)]
pub(super) struct PreviewProcessed;

/// Scene that used for preview generation.
#[derive(Bundle)]
struct PreviewSceneBundle {
    name: Name,
    preview_target: PreviewTarget,
    scene_bundle: SceneBundle,
}

impl PreviewSceneBundle {
    fn new(translation: Vec3, scene_handle: Handle<Scene>, preview_entity: Entity) -> Self {
        Self {
            name: "Preview scene".into(),
            preview_target: PreviewTarget(preview_entity),
            scene_bundle: SceneBundle {
                scene: scene_handle,
                transform: Transform::from_translation(translation)
                    .with_rotation(Quat::from_rotation_y(PI)), // Rotate towards camera.
                ..Default::default()
            },
        }
    }
}

/// Points to the entity for which the preview will be generated.
#[derive(Component)]
struct PreviewTarget(Entity);
