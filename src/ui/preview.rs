use std::f32::consts::PI;

use bevy::{
    asset::{HandleId, LoadState},
    prelude::*,
    render::{
        camera::RenderTarget,
        render_resource::{Extent3d, TextureUsages},
        view::{NoFrustumCulling, RenderLayers},
    },
};
use bevy_scene_hook::{HookedSceneBundle, SceneHook};

use crate::core::asset_metadata::{self, ObjectMetadata};

pub(super) struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<PreviewState>()
            .add_startup_system(Self::spawn_camera_system)
            .add_systems((
                Self::deactivation_system.in_schedule(OnEnter(PreviewState::Inactive)),
                Self::scene_spawning_system.in_set(OnUpdate(PreviewState::Inactive)),
                Self::loading_system.in_set(OnUpdate(PreviewState::LoadingAsset)),
                Self::finish_system.in_schedule(OnEnter(PreviewState::Rendering)),
            ));
    }
}

impl PreviewPlugin {
    fn spawn_camera_system(mut commands: Commands) {
        commands.spawn(PreviewCameraBundle::default());
    }

    fn scene_spawning_system(
        mut commands: Commands,
        mut preview_state: ResMut<NextState<PreviewState>>,
        asset_server: Res<AssetServer>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        previews: Query<(Entity, &Preview), Without<PreviewProcessed>>,
        parents: Query<&Parent>,
        styles: Query<&Style>,
        actors: Query<&Handle<Scene>>,
        preview_cameras: Query<Entity, With<PreviewCamera>>,
    ) {
        if let Some((preview_entity, preview)) = previews.iter().find(|&(entity, _)| {
            styles
                .iter_many(parents.iter_ancestors(entity))
                .all(|style| style.display == Display::Flex)
        }) {
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

    fn loading_system(
        mut commands: Commands,
        mut asset_events: EventWriter<AssetEvent<Image>>,
        mut preview_state: ResMut<NextState<PreviewState>>,
        mut images: ResMut<Assets<Image>>,
        asset_server: Res<AssetServer>,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_scenes: Query<(&PreviewTarget, &Handle<Scene>)>,
        previews: Query<&Preview>,
    ) {
        let (preview_target, scene_handle) = preview_scenes.single();
        match asset_server.get_load_state(scene_handle) {
            LoadState::NotLoaded | LoadState::Loading => (),
            LoadState::Loaded => {
                debug!("asset for preview was sucessfully loaded");

                let Ok(preview) = previews.get(preview_target.0) else {
                    // Entity target is longer valid.
                    preview_state.set(PreviewState::Inactive);
                    return;
                };

                let mut image = Image::default();
                image.texture_descriptor.usage |= TextureUsages::RENDER_ATTACHMENT;
                image.resize(Extent3d {
                    width: preview.width,
                    height: preview.height,
                    ..Default::default()
                });

                let image_handle = images.add(image);
                commands
                    .entity(preview_target.0)
                    .insert(image_handle.clone());

                // A workaround for this bug: https://github.com/bevyengine/bevy/issues/5595.
                asset_events.send(AssetEvent::Modified {
                    handle: image_handle.clone(),
                });

                let mut camera = preview_cameras.single_mut();
                camera.is_active = true;
                camera.target = RenderTarget::Image(image_handle);

                preview_state.set(PreviewState::Rendering);
            }
            LoadState::Failed => {
                error!("unable to load asset for preview");

                preview_state.set(PreviewState::Inactive);
            }
            LoadState::Unloaded => {
                unreachable!("asset for preview shouldn't be unloaded");
            }
        }
    }

    fn finish_system(mut preview_state: ResMut<NextState<PreviewState>>) {
        debug!("requested inactive state after rendering");
        preview_state.set(PreviewState::Inactive);
    }

    fn deactivation_system(
        mut commands: Commands,
        mut preview_cameras: Query<&mut Camera, With<PreviewCamera>>,
        preview_scenes: Query<Entity, With<PreviewTarget>>,
    ) {
        if let Ok(entity) = preview_scenes.get_single() {
            commands.entity(entity).despawn_recursive();
        }
        preview_cameras.single_mut().is_active = false;
    }
}

const PREVIEW_RENDER_LAYER: RenderLayers = RenderLayers::layer(1);

#[derive(Bundle)]
struct PreviewCameraBundle {
    name: Name,
    preview_camera: PreviewCamera,
    render_layer: RenderLayers,
    ui_config: UiCameraConfig,
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
                camera: Camera {
                    is_active: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            ui_config: UiCameraConfig { show_ui: false },
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

/// Contains information about the preview, generated image handle will be added as a child.
///
/// Preview generation happens only if UI element entity is visible.
/// Processed entities will be marked with [`PreviewProcessed`].
#[derive(Component)]
pub(crate) struct Preview {
    kind: PreviewKind,
    width: u32,
    height: u32,
}

impl Preview {
    pub(crate) fn object(id: HandleId, size: Size) -> Self {
        Self::new(PreviewKind::Object(id), size)
    }

    pub(crate) fn actor(entity: Entity, size: Size) -> Self {
        Self::new(PreviewKind::Actor(entity), size)
    }

    fn new(kind: PreviewKind, size: Size) -> Self {
        let Val::Px(width) = size.width else {
            panic!("width should be in pixels");
        };
        let Val::Px(height) = size.height else {
            panic!("height should be in pixels");
        };

        Self {
            kind,
            width: width as u32,
            height: height as u32,
        }
    }
}

/// Points to the asset or object for which the preview will be generated.
enum PreviewKind {
    /// Actor entity.
    Actor(Entity),
    /// Asset's metadata ID.
    Object(HandleId),
}

/// Marks entity with [`Preview`] as processed end excludes it from preview generation.
#[derive(Component)]
struct PreviewProcessed;

/// Scene that used for preview generation.
#[derive(Bundle)]
struct PreviewSceneBundle {
    name: Name,
    preview_target: PreviewTarget,
    scene: HookedSceneBundle,
}

impl PreviewSceneBundle {
    fn new(translation: Vec3, scene_handle: Handle<Scene>, preview_entity: Entity) -> Self {
        Self {
            name: "Preview scene".into(),
            preview_target: PreviewTarget(preview_entity),
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

/// Points to the entity for which the preview will be generated.
#[derive(Component)]
struct PreviewTarget(Entity);
