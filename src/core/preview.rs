use bevy::{
    asset::{HandleId, LoadState},
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureUsages},
        view::RenderLayers,
    },
    utils::HashMap,
};
use bevy_egui::{egui::TextureId, EguiContexts};
use bevy_scene_hook::{HookedSceneBundle, SceneHook};

use super::asset_metadata::{self, ObjectMetadata};

pub(super) struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<PreviewState>()
            .add_event::<PreviewRequest>()
            .init_resource::<Previews>()
            .add_startup_system(Self::spawn_camera_system)
            .add_systems((
                Self::deactivation_system.in_schedule(OnEnter(PreviewState::Inactive)),
                Self::load_asset_system.in_set(OnUpdate(PreviewState::Inactive)),
                Self::wait_for_loading_system.in_set(OnUpdate(PreviewState::LoadingAsset)),
                Self::finish_rendering_system.in_schedule(OnEnter(PreviewState::Rendering)),
            ));
    }
}

pub(crate) const PREVIEW_SIZE: u32 = 64;

impl PreviewPlugin {
    fn spawn_camera_system(mut commands: Commands) {
        commands.spawn(PreviewCameraBundle::default());
    }

    fn load_asset_system(
        mut commands: Commands,
        mut preview_events: EventReader<PreviewRequest>,
        mut preview_state: ResMut<NextState<PreviewState>>,
        asset_server: Res<AssetServer>,
        previews: Res<Previews>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        preview_cameras: Query<Entity, With<PreviewCamera>>,
    ) {
        if let Some(preview_event) = preview_events
            .iter()
            .find(|preview_event| !previews.contains_key(&preview_event.0))
        {
            let metadata_handle = object_metadata.get_handle(preview_event.0);
            let preview_translation = object_metadata
                .get(&metadata_handle)
                .map(|metadata| metadata.general.preview_translation)
                .expect("preview event handle should be a metadata handle");

            let metadata_path = asset_server
                .get_handle_path(preview_event.0)
                .expect("metadata handle should have a path");
            let scene_path = asset_metadata::scene_path(metadata_path.path());
            let scene_handle = asset_server.load(&scene_path);

            debug!("loading {scene_path} to generate preview");

            commands
                .entity(preview_cameras.single())
                .with_children(|parent| {
                    parent.spawn(PreviewTargetBundle::new(
                        preview_translation,
                        scene_handle,
                        preview_event.0,
                    ));
                });

            preview_state.set(PreviewState::LoadingAsset);
        }
        preview_events.clear();
    }

    fn wait_for_loading_system(
        mut egui: EguiContexts,
        mut asset_events: EventWriter<AssetEvent<Image>>,
        mut previews: ResMut<Previews>,
        mut preview_state: ResMut<NextState<PreviewState>>,
        mut images: ResMut<Assets<Image>>,
        asset_server: Res<AssetServer>,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_target: Query<(&PreviewMetadataId, &Handle<Scene>)>,
    ) {
        let mut camera = preview_cameras.single_mut();
        let (metadata_id, scene_handle) = preview_target.single();
        match asset_server.get_load_state(scene_handle) {
            LoadState::NotLoaded | LoadState::Loading => (),
            LoadState::Loaded => {
                debug!(
                    "asset {:?} was sucessfully loaded to generate preview",
                    asset_server.get_handle_path(metadata_id.0)
                );

                let mut image = Image::default();
                image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
                image.resize(Extent3d {
                    width: PREVIEW_SIZE,
                    height: PREVIEW_SIZE,
                    ..Default::default()
                });

                let image_handle = images.add(image);
                let texture_id = egui.add_image(image_handle.clone());
                previews.insert(metadata_id.0, texture_id);

                // A workaround for this bug: https://github.com/bevyengine/bevy/issues/5595
                asset_events.send(AssetEvent::Modified {
                    handle: image_handle.clone(),
                });

                camera.is_active = true;
                camera.target = RenderTarget::Image(image_handle);
                preview_state.set(PreviewState::Rendering);
            }
            LoadState::Failed => {
                error!(
                    "unable to load preview for {:?}",
                    asset_server.get_handle_path(metadata_id.0)
                );

                previews.insert(metadata_id.0, TextureId::Managed(0));
                preview_state.set(PreviewState::Inactive);
            }
            LoadState::Unloaded => {
                unreachable!(
                    "asset {:?} shouldn't be unloaded during the generating preview",
                    asset_server.get_handle_path(metadata_id.0)
                );
            }
        }
    }

    fn finish_rendering_system(mut preview_state: ResMut<NextState<PreviewState>>) {
        debug!("requested inactive state after rendering");
        preview_state.set(PreviewState::Inactive);
    }

    fn deactivation_system(
        mut commands: Commands,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_targets: Query<Entity, With<PreviewMetadataId>>,
    ) {
        if let Ok(entity) = preview_targets.get_single() {
            commands.entity(entity).despawn_recursive();
        }
        preview_cameras.single_mut().is_active = false;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, States)]
enum PreviewState {
    #[default]
    Inactive,
    LoadingAsset,
    Rendering,
}

/// An event that indicates a request for preview for an asset.
/// Contains the metadata handle of this asset.
pub(crate) struct PreviewRequest(pub(crate) HandleId);

/// Maps metadata handles to preview image handles.
#[derive(Default, Deref, DerefMut, Resource)]
pub(crate) struct Previews(HashMap<HandleId, TextureId>);

const PREVIEW_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Bundle)]
struct PreviewCameraBundle {
    name: Name,
    preview_camera: PreviewCamera,
    render_layer: RenderLayers,

    #[bundle]
    camera_bundle: Camera3dBundle,

    #[bundle]
    visibility_bundle: VisibilityBundle,
}

impl Default for PreviewCameraBundle {
    fn default() -> Self {
        Self {
            name: "Preview camera".into(),
            preview_camera: PreviewCamera,
            render_layer: PREVIEW_RENDER_LAYER,
            camera_bundle: Camera3dBundle {
                camera: Camera {
                    order: -1,
                    is_active: false,
                    ..Default::default()
                },
                // Place a little upper to avoid overlapping lights with the ground,
                // since the light sources are shared beteween layers (https://github.com/bevyengine/bevy/issues/3462).
                transform: Transform::from_translation(10.0 * Vec3::Y),
                ..Default::default()
            },
            visibility_bundle: Default::default(),
        }
    }
}

/// Indicates that a camera is used for generating previews.
#[derive(Component)]
struct PreviewCamera;

#[derive(Bundle)]
struct PreviewTargetBundle {
    name: Name,
    metadata_id: PreviewMetadataId,

    #[bundle]
    scene: HookedSceneBundle,
}

impl PreviewTargetBundle {
    fn new(translation: Vec3, preview_handle: Handle<Scene>, metadata_id: HandleId) -> Self {
        Self {
            name: "Preview target".into(),
            metadata_id: PreviewMetadataId(metadata_id),
            scene: HookedSceneBundle {
                scene: SceneBundle {
                    scene: preview_handle,
                    // Keep object a little far from the camera
                    transform: Transform::from_translation(translation),
                    ..Default::default()
                },
                hook: SceneHook::new(|entity, commands| {
                    if entity.contains::<Handle<Mesh>>() {
                        commands.insert(PREVIEW_RENDER_LAYER);
                    }
                }),
            },
        }
    }
}

/// Stores a handle ID to the preview asset's metadata.
#[derive(Component)]
struct PreviewMetadataId(HandleId);
