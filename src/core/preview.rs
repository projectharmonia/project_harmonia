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
use bevy_egui::EguiContext;
use bevy_inspector_egui::egui::TextureId;
use bevy_scene_hook::{HookedSceneBundle, SceneHook};
use derive_more::From;
use iyes_loopless::prelude::*;
use strum::Display;

use super::{
    asset_metadata::{self, AssetMetadata},
    game_world::GameWorld,
};

pub(crate) const PREVIEW_SIZE: u32 = 64;

pub(super) struct PreviewPlugin;

impl Plugin for PreviewPlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(PreviewState::Inactive)
            .add_event::<PreviewRequested>()
            .init_resource::<Previews>()
            .add_startup_system(Self::spawn_camera_system)
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>())
            .add_enter_system(PreviewState::Inactive, Self::deactivation_system)
            .add_system(Self::load_asset_system.run_in_state(PreviewState::Inactive))
            .add_system(Self::wait_for_loading_system.run_in_state(PreviewState::LoadingAsset))
            .add_enter_system(PreviewState::Rendering, Self::finish_rendering_system);
    }
}

impl PreviewPlugin {
    fn spawn_camera_system(mut commands: Commands) {
        commands.spawn_bundle(PreviewCameraBundle::default());
    }

    fn cleanup_system(mut commands: Commands, preview_cameras: Query<Entity, With<PreviewCamera>>) {
        for entity in &preview_cameras {
            commands.entity(entity).despawn_recursive();
        }
    }

    fn load_asset_system(
        mut commands: Commands,
        mut preview_events: EventReader<PreviewRequested>,
        asset_server: Res<AssetServer>,
        previews: Res<Previews>,
        metadata: Res<Assets<AssetMetadata>>,
        preview_cameras: Query<Entity, With<PreviewCamera>>,
    ) {
        if let Some(preview_event) = preview_events
            .iter()
            .find(|preview_event| !previews.contains_key(&preview_event.0))
        {
            let metadata_handle = metadata.get_handle(preview_event.0);
            let metadata = metadata
                .get(&metadata_handle)
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
                    parent.spawn_bundle(PreviewTargetBundle::new(
                        metadata.general.preview_translation,
                        scene_handle,
                        preview_event.0,
                    ));
                });

            commands.insert_resource(NextState(PreviewState::LoadingAsset));
        }
        preview_events.clear();
    }

    #[cfg_attr(coverage, no_coverage)]
    fn wait_for_loading_system(
        mut commands: Commands,
        mut asset_events: EventWriter<AssetEvent<Image>>,
        mut previews: ResMut<Previews>,
        mut egui: ResMut<EguiContext>,
        mut images: ResMut<Assets<Image>>,
        asset_server: Res<AssetServer>,
        mut preview_camera: Query<&mut Camera, With<PreviewCamera>>,
        preview_target: Query<(&PreviewMetadataId, &Handle<Scene>)>,
    ) {
        let mut camera = preview_camera.single_mut();
        let (metadata_id, scene_handle) = preview_target.single();
        match asset_server.get_load_state(scene_handle) {
            LoadState::NotLoaded => unreachable!(
                "asset {:?} should have started loading when entering {} state",
                asset_server.get_handle_path(metadata_id.0),
                PreviewState::LoadingAsset
            ),
            LoadState::Loading => (),
            LoadState::Loaded => {
                // Ignore logging in tests to exclude it from coverage.
                #[cfg(not(test))]
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
                commands.insert_resource(NextState(PreviewState::Rendering));
            }
            LoadState::Failed => {
                // Ignore logging in tests to exclude it from coverage.
                #[cfg(not(test))]
                error!(
                    "unable to load preview for {:?}",
                    asset_server.get_handle_path(metadata_id.0)
                );

                previews.insert(metadata_id.0, TextureId::Managed(0));
                commands.insert_resource(NextState(PreviewState::Inactive));
            }
            LoadState::Unloaded => {
                unreachable!(
                    "asset {:?} shouldn't be unloaded during the generating preview",
                    asset_server.get_handle_path(metadata_id.0)
                );
            }
        }
    }

    fn finish_rendering_system(mut commands: Commands) {
        debug!("requested inactive state after rendering");
        commands.insert_resource(NextState(PreviewState::Inactive));
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

#[derive(Clone, Copy, Debug, Display, Eq, Hash, PartialEq)]
enum PreviewState {
    Inactive,
    LoadingAsset,
    Rendering,
}

/// An event that indicates a request to preview an asset.
/// Contains the metadata handle of this asset.
pub(crate) struct PreviewRequested(pub(crate) HandleId);

/// Maps metadata handles to preview image handles.
#[derive(Default, Deref, DerefMut)]
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
                    priority: -1,
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
pub(crate) struct PreviewCamera;

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
            metadata_id: metadata_id.into(),
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
#[derive(Component, From)]
struct PreviewMetadataId(HandleId);

#[cfg(test)]
mod tests {
    use bevy::{
        gltf::GltfPlugin, input::InputPlugin, scene::ScenePlugin, time::TimePlugin, utils::Uuid,
    };
    use bevy_egui::EguiPlugin;
    use bevy_scene_hook::HookPlugin;

    use super::*;
    use crate::core::{
        asset_metadata::{self, AssetMetadata, AssetMetadataLoader},
        tests::{self, HeadlessRenderPlugin},
    };

    const METADATA_PATH: &str = "base/objects/rocks/small_stone/small_stone.toml";

    #[test]
    fn cleanup() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_plugin(TestPreviewPlugin);

        app.update();

        let preview_camera_entity = app
            .world
            .query_filtered::<Entity, With<PreviewCamera>>()
            .single(&app.world);
        let preview_target_entity = app.world.spawn().id();
        app.world
            .entity_mut(preview_camera_entity)
            .push_children(&[preview_target_entity]);

        app.world.remove_resource::<GameWorld>();

        app.update();

        assert!(app.world.get_entity(preview_target_entity).is_none());
        assert!(app.world.get_entity(preview_camera_entity).is_none());
    }

    #[test]
    fn preview_event() {
        let mut app = App::new();
        app.add_plugin(TestPreviewPlugin);

        let asset_server = app.world.resource::<AssetServer>();
        let metadata_handle: Handle<AssetMetadata> = asset_server.load(METADATA_PATH);

        tests::wait_for_asset_loading(&mut app, &metadata_handle);

        let mut events = app.world.resource_mut::<Events<PreviewRequested>>();
        events.send(PreviewRequested(metadata_handle.id));

        app.update();

        assert_eq!(
            app.world.resource::<NextState<PreviewState>>().0,
            PreviewState::LoadingAsset,
        );
        assert_eq!(
            app.world.query::<&PreviewMetadataId>().single(&app.world).0,
            metadata_handle.id,
        );
    }

    #[test]
    fn asset_loading() {
        let mut app = App::new();
        app.add_plugin(TestPreviewPlugin).add_plugin(HookPlugin);

        app.update();

        let asset_server = app.world.resource::<AssetServer>();
        let metadata_handle: Handle<AssetMetadata> = asset_server.load(METADATA_PATH);
        let preview_handle: Handle<Scene> =
            asset_server.load(&asset_metadata::scene_path(METADATA_PATH));

        let camera_entity = app
            .world
            .query_filtered::<Entity, With<Camera>>()
            .single(&app.world);
        app.world.entity_mut(camera_entity).with_children(|parent| {
            parent.spawn().insert_bundle(PreviewTargetBundle::new(
                Vec3::default(),
                preview_handle.clone(),
                metadata_handle.id,
            ));
        });

        app.insert_resource(NextState(PreviewState::LoadingAsset));

        tests::wait_for_asset_loading(&mut app, &preview_handle);

        assert_eq!(
            app.world.resource::<NextState<PreviewState>>().0,
            PreviewState::Rendering,
        );

        let render_layer = *app
            .world
            .query_filtered::<&RenderLayers, With<Handle<Mesh>>>()
            .single(&app.world);
        assert_eq!(render_layer, PREVIEW_RENDER_LAYER);
    }

    #[test]
    fn rendering_frame() {
        let mut app = App::new();
        app.add_plugin(TestPreviewPlugin);

        app.update();

        let preview_entity = app
            .world
            .spawn()
            .insert(PreviewMetadataId(HandleId::Id(Uuid::nil(), 0)))
            .id();
        let camera_entity = app
            .world
            .query_filtered::<Entity, With<Camera>>()
            .single(&app.world);
        app.world
            .entity_mut(camera_entity)
            .push_children(&[preview_entity]);

        app.insert_resource(NextState(PreviewState::Rendering));

        app.update();

        assert_eq!(
            app.world.resource::<CurrentState<PreviewState>>().0,
            PreviewState::Inactive,
        );
        assert!(app.world.get_entity(preview_entity).is_none());
    }

    struct TestPreviewPlugin;

    impl Plugin for TestPreviewPlugin {
        fn build(&self, app: &mut App) {
            app.add_plugin(HeadlessRenderPlugin)
                .init_asset_loader::<AssetMetadataLoader>()
                .add_asset::<AssetMetadata>()
                .add_plugin(ScenePlugin)
                .add_plugin(InputPlugin)
                .add_plugin(TimePlugin)
                .add_plugin(GltfPlugin)
                .add_plugin(TransformPlugin)
                .add_plugin(HierarchyPlugin)
                .add_plugin(EguiPlugin)
                .add_plugin(PreviewPlugin);
        }
    }
}
