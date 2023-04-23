use std::f32::consts::PI;

use bevy::{
    asset::{HandleId, LoadState},
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureUsages},
        view::{NoFrustumCulling, RenderLayers},
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

impl PreviewPlugin {
    fn spawn_camera_system(mut commands: Commands) {
        commands.spawn(PreviewCameraBundle::default());
    }

    fn load_asset_system(
        mut commands: Commands,
        mut preview_state: ResMut<NextState<PreviewState>>,
        asset_server: Res<AssetServer>,
        mut previews: ResMut<Previews>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        preview_cameras: Query<Entity, With<PreviewCamera>>,
        actors: Query<&Handle<Scene>>,
    ) {
        if let Some(preview) = previews.requested.take() {
            let (translation, scene_handle) = match preview.kind {
                PreviewKind::Actor(entity) => {
                    debug!("generating preview for actor {entity:?}");
                    let scene_handle = actors
                        .get(entity)
                        .expect("actor for preview should have a scene handle");
                    (Vec3::new(0.0, -1.67, -0.42), scene_handle.clone())
                }
                PreviewKind::Object(id) => {
                    let object_metadata = object_metadata
                        .get(&object_metadata.get_handle(id))
                        .expect("preview event handle should be a metadata handle");
                    let metadata_path = asset_server
                        .get_handle_path(id)
                        .expect("metadata handle should have a path");
                    debug!("generating preview for {metadata_path:?}");

                    let scene_handle = asset_server.load(asset_metadata::scene_path(metadata_path));
                    (object_metadata.general.preview_translation, scene_handle)
                }
            };

            commands
                .entity(preview_cameras.single())
                .with_children(|parent| {
                    parent.spawn(PreviewTargetBundle::new(translation, scene_handle, preview));
                });

            preview_state.set(PreviewState::LoadingAsset);
        }
    }

    fn wait_for_loading_system(
        mut egui: EguiContexts,
        mut asset_events: EventWriter<AssetEvent<Image>>,
        mut previews: ResMut<Previews>,
        mut preview_state: ResMut<NextState<PreviewState>>,
        mut images: ResMut<Assets<Image>>,
        asset_server: Res<AssetServer>,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_target: Query<(&Preview, &Handle<Scene>)>,
    ) {
        let mut camera = preview_cameras.single_mut();
        let (&preview, scene_handle) = preview_target.single();
        match asset_server.get_load_state(scene_handle) {
            LoadState::NotLoaded | LoadState::Loading => (),
            LoadState::Loaded => {
                debug!("asset for preview was sucessfully loaded");

                let mut image = Image::default();
                image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
                image.resize(Extent3d {
                    width: preview.size,
                    height: preview.size,
                    ..Default::default()
                });

                let image_handle = images.add(image);
                let texture_id = egui.add_image(image_handle.clone());
                previews.generated.insert(preview, texture_id);

                // A workaround for this bug: https://github.com/bevyengine/bevy/issues/5595
                asset_events.send(AssetEvent::Modified {
                    handle: image_handle.clone(),
                });

                camera.is_active = true;
                camera.target = RenderTarget::Image(image_handle);
                preview_state.set(PreviewState::Rendering);
            }
            LoadState::Failed => {
                error!("unable to load asset for preview");

                previews.generated.insert(preview, TextureId::Managed(0));
                preview_state.set(PreviewState::Inactive);
            }
            LoadState::Unloaded => {
                unreachable!("asset for preview shouldn't be unloaded");
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
        preview_targets: Query<Entity, With<Preview>>,
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

#[derive(Default, Resource)]
pub(crate) struct Previews {
    generated: HashMap<Preview, TextureId>,
    requested: Option<Preview>,
}

impl Previews {
    /// Returns a texture ID for specified preview.
    ///
    /// If the preview has not yet been generated, it returns
    /// placeholder texture and requests the generation.
    /// Generates only one preview at a time.
    /// Other previews will be discarded and have to be re-scheduled again.
    pub(crate) fn get(&mut self, preview: Preview) -> TextureId {
        if let Some(texture_id) = self.generated.get(&preview) {
            return *texture_id;
        }

        if self.requested.is_none() {
            self.requested = Some(preview);
        }

        TextureId::Managed(0)
    }
}

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
    preview: Preview,

    #[bundle]
    scene: HookedSceneBundle,
}

impl PreviewTargetBundle {
    fn new(translation: Vec3, scene_handle: Handle<Scene>, preview: Preview) -> Self {
        Self {
            name: "Preview target".into(),
            preview,
            scene: HookedSceneBundle {
                scene: SceneBundle {
                    scene: scene_handle,
                    transform: Transform::from_translation(translation)
                        .with_rotation(Quat::from_rotation_y(PI)), // Rotate towards camera.
                    ..Default::default()
                },
                hook: SceneHook::new(|entity, commands| {
                    if entity.contains::<Handle<Mesh>>() {
                        commands.insert((PREVIEW_RENDER_LAYER, NoFrustumCulling));
                    }
                }),
            },
        }
    }
}

/// Stores information about the preview.
///
/// Used to get or generate a new preview from [`Previews`] resource.
/// Also used as a component to mark entity as a preview entity.
#[derive(Clone, Component, Copy, Eq, Hash, PartialEq)]
pub(crate) struct Preview {
    pub(crate) kind: PreviewKind,
    pub(crate) size: u32,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub(crate) enum PreviewKind {
    /// Actor entity.
    Actor(Entity),
    /// Asset's metadata ID.
    Object(HandleId),
}
