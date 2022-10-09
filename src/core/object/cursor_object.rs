use std::{f32::consts::FRAC_PI_4, path::PathBuf};

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_mod_raycast::Ray3d;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use tap::TapOptional;

use crate::core::{
    asset_metadata,
    city::City,
    control_action::ControlAction,
    game_state::GameState,
    network::network_event::{
        client_event::{ClientEvent, ClientEventAppExt, ClientSendBuffer},
        server_event::{SendMode, ServerEvent, ServerEventAppExt, ServerSendBuffer},
    },
    preview::PreviewCamera,
};

use super::{ObjectBundle, PickCancelled, Picked};

pub(super) struct CursorObjectPlugin;

impl Plugin for CursorObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_client_event::<ObjectMoved>()
            .add_client_event::<ObjectSpawned>()
            .add_server_event::<SpawnConfirmed>()
            .add_system(Self::spawning_system.run_in_state(GameState::City))
            .add_system(Self::movement_system.run_in_state(GameState::City))
            .add_system(
                Self::confirm_system
                    .run_in_state(GameState::City)
                    .run_if(is_placement_confirmed),
            )
            .add_system(
                Self::cancel_system
                    .run_in_state(GameState::City)
                    .run_if(is_placement_canceled),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::despawn_system.run_in_state(GameState::City),
            )
            .add_system(Self::apply_movement_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::spawn_object_system.run_unless_resource_exists::<RenetClient>());
    }
}

impl CursorObjectPlugin {
    fn spawning_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut objects: Query<(&Transform, &Handle<Scene>, &mut Visibility)>,
        new_cursor_objects: Query<(Entity, &CursorObject), Added<CursorObject>>,
    ) {
        for (cursor_entity, cursor_object) in &new_cursor_objects {
            debug!("created cursor {cursor_object:?}");
            match cursor_object {
                CursorObject::Spawning(metadata_path) => {
                    commands
                        .entity(cursor_entity)
                        .insert_bundle(SceneBundle {
                            scene: asset_server.load(&asset_metadata::scene_path(&metadata_path)),
                            ..Default::default()
                        })
                        .insert(CursorOffset::default());
                }
                CursorObject::Moving(object_entity) => {
                    let (transform, scene_handle, mut visibility) = objects
                        .get_mut(*object_entity)
                        .expect("moving object should exist with these components");
                    commands.entity(cursor_entity).insert_bundle(SceneBundle {
                        scene: scene_handle.clone(),
                        transform: *transform,
                        ..Default::default()
                    });
                    visibility.is_visible = false;
                }
            }
        }
    }

    fn movement_system(
        mut commands: Commands,
        windows: Res<Windows>,
        rapier_ctx: Res<RapierContext>,
        action_state: Res<ActionState<ControlAction>>,
        camera: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
        mut cursor_objects: Query<
            (Entity, &mut Transform, Option<&CursorOffset>),
            With<CursorObject>,
        >,
    ) {
        if let Ok((entity, mut transform, cursor_offset)) = cursor_objects.get_single_mut() {
            if let Some(cursor_pos) = windows
                .get_primary()
                .and_then(|window| window.cursor_position())
            {
                let (&camera_transform, camera) = camera.single();
                let ray = Ray3d::from_screenspace(cursor_pos, camera, &camera_transform)
                    .expect("ray should be created from screen coordinates");

                let toi = rapier_ctx
                    .cast_ray(
                        ray.origin(),
                        ray.direction(),
                        f32::MAX,
                        false,
                        QueryFilter::new(),
                    )
                    .map(|(_, toi)| toi)
                    .unwrap_or_default();

                let ray_translation = ray.origin() + ray.direction() * toi;
                let offset = cursor_offset.copied().unwrap_or_else(|| {
                    let offset = CursorOffset(transform.translation.xz() - ray_translation.xz());
                    commands.entity(entity).insert(offset);
                    offset
                });
                transform.translation = ray_translation + Vec3::new(offset.x, 0.0, offset.y);
                if action_state.just_pressed(ControlAction::RotateObject) {
                    const ROTATION_STEP: f32 = -FRAC_PI_4;
                    transform.rotate_y(ROTATION_STEP);
                }
            }
        }
    }

    fn confirm_system(
        mut move_buffers: ResMut<ClientSendBuffer<ObjectMoved>>,
        mut spawn_events: ResMut<ClientSendBuffer<ObjectSpawned>>,
        cursor_objects: Query<(&Transform, &CursorObject)>,
    ) {
        if let Ok((transform, cursor_object)) = cursor_objects.get_single() {
            debug!("confirmed cursor {cursor_object:?}");
            match cursor_object {
                CursorObject::Spawning(metadata_path) => spawn_events.push(ObjectSpawned {
                    metadata_path: metadata_path.clone(),
                    translation: transform.translation,
                    rotation: transform.rotation,
                }),
                CursorObject::Moving(_) => move_buffers.push(ObjectMoved {
                    translation: transform.translation,
                    rotation: transform.rotation,
                }),
            }
        }
    }

    fn cancel_system(
        mut commands: Commands,
        mut cancel_buffer: ResMut<ClientSendBuffer<PickCancelled>>,
        moving_objects: Query<(Entity, &CursorObject)>,
    ) {
        if let Ok((entity, cursor_object)) = moving_objects.get_single() {
            debug!("cancelled cursor {cursor_object:?}");
            if let CursorObject::Spawning(_) = cursor_object {
                commands.entity(entity).despawn_recursive();
            } else {
                cancel_buffer.push(PickCancelled);
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        pick_removals: RemovedComponents<Picked>,
        mut spawn_confirm_events: EventReader<SpawnConfirmed>,
        cursor_objects: Query<(Entity, &CursorObject)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        if let Ok((cursor_entity, cursor_object)) = cursor_objects.get_single() {
            match *cursor_object {
                CursorObject::Spawning(_) => {
                    if spawn_confirm_events.iter().count() != 0 {
                        debug!("despawned cursor {cursor_object:?}");
                        commands.entity(cursor_entity).despawn_recursive();
                    }
                }
                CursorObject::Moving(moving_entity) => {
                    if let Some(object_entity) = pick_removals
                        .iter()
                        .find(|&object_entity| object_entity == moving_entity)
                    {
                        debug!("despawned cursor {cursor_object:?}");
                        commands.entity(cursor_entity).despawn_recursive();
                        let mut visibility = visibility
                            .get_mut(object_entity)
                            .expect("object should have visibility");
                        visibility.is_visible = true;
                    }
                }
            }
        }
    }

    fn spawn_object_system(
        mut commands: Commands,
        mut spawn_events: EventReader<ClientEvent<ObjectSpawned>>,
        mut spawn_confirm_buffer: ResMut<ServerSendBuffer<SpawnConfirmed>>,
        visible_cities: Query<Entity, (With<City>, With<Visibility>)>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.iter().cloned() {
            commands.spawn_bundle(ObjectBundle::new(
                event.metadata_path,
                event.translation,
                event.rotation,
                visible_cities.single(),
            ));
            spawn_confirm_buffer.push(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: SpawnConfirmed,
            });
        }
    }

    fn apply_movement_system(
        mut commands: Commands,
        mut move_events: EventReader<ClientEvent<ObjectMoved>>,
        mut picked_objects: Query<(Entity, &mut Transform, &Picked)>,
    ) {
        for ClientEvent { client_id, event } in move_events.iter().copied() {
            if let Some((entity, mut transform, ..)) = picked_objects
                .iter_mut()
                .find(|(.., picked)| picked.0 == client_id)
                .tap_none(|| error!("unable to map received entity"))
            {
                transform.translation = event.translation;
                transform.rotation = event.rotation;
                commands.entity(entity).remove::<Picked>();
            }
        }
    }
}

fn is_placement_canceled(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.just_pressed(ControlAction::Cancel)
}

fn is_placement_confirmed(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.just_pressed(ControlAction::Confirm)
}

pub(crate) fn is_cursor_object_exists(cursor_objects: Query<(), With<CursorObject>>) -> bool {
    !cursor_objects.is_empty()
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ObjectMoved {
    translation: Vec3,
    rotation: Quat,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ObjectSpawned {
    metadata_path: PathBuf,
    translation: Vec3,
    rotation: Quat,
}

#[derive(Deserialize, Serialize, Debug)]
struct SpawnConfirmed;

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug)]
pub(crate) enum CursorObject {
    Spawning(PathBuf),
    Moving(Entity),
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy::{asset::AssetPlugin, core::CorePlugin};
    use itertools::Itertools;

    use super::*;
    use crate::core::{game_world::parent_sync::ParentSync, object::ObjectPath};

    #[test]
    fn spawning_cursor_spawning() {
        let mut app = App::new();
        app.add_plugin(TestCursorObjectPlugin);

        let cursor_entity = app
            .world
            .spawn()
            .insert(CursorObject::Spawning(PathBuf::new()))
            .id();

        app.update();

        assert!(app.world.get::<Handle<Scene>>(cursor_entity).is_some());
    }

    #[test]
    fn moving_cursor_spawning() {
        let mut app = App::new();
        app.add_plugin(TestCursorObjectPlugin);

        let object_bundle = DummyObjectBundle::default();
        let object_entity = app.world.spawn().insert_bundle(object_bundle.clone()).id();
        let cursor_entity = app
            .world
            .spawn()
            .insert(CursorObject::Moving(object_entity))
            .id();

        app.update();

        let visibility = app.world.get::<Visibility>(object_entity).unwrap();
        assert!(!visibility.is_visible);

        let cursor_entity = app.world.entity(cursor_entity);
        assert_eq!(
            *cursor_entity.get::<Transform>().unwrap(),
            object_bundle.transform
        );
        assert_eq!(
            *cursor_entity.get::<Handle<Scene>>().unwrap(),
            object_bundle.scene
        );
    }

    #[test]
    fn spawning_cursor_confirmation() {
        let mut app = App::new();
        app.add_plugin(TestCursorObjectPlugin);

        const TRANSFORM: Transform =
            Transform::from_translation(Vec3::ONE).with_rotation(Quat::from_array([1.0; 4]));
        const METADATA_PATH: &str = "dummy";
        app.world
            .spawn()
            .insert(TRANSFORM)
            .insert(CursorObject::Spawning(METADATA_PATH.into()));

        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::Confirm);

        app.update();

        let spawn_buffer = app.world.resource::<ClientSendBuffer<ObjectSpawned>>();
        let spawn_event = spawn_buffer.iter().exactly_one().unwrap();
        assert_eq!(spawn_event.metadata_path, Path::new(METADATA_PATH));
        assert_eq!(spawn_event.translation, TRANSFORM.translation);
        assert_eq!(spawn_event.rotation, TRANSFORM.rotation);
    }

    #[test]
    fn moving_cursor_confirmation() {
        let mut app = App::new();
        app.add_plugin(TestCursorObjectPlugin);

        const TRANSFORM: Transform =
            Transform::from_translation(Vec3::ONE).with_rotation(Quat::from_array([1.0; 4]));
        let object_entity = app
            .world
            .spawn()
            .insert_bundle(DummyObjectBundle {
                transform: TRANSFORM,
                ..Default::default()
            })
            .id();
        app.world
            .spawn()
            .insert(CursorObject::Moving(object_entity));

        app.update();

        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::Confirm);

        app.update();

        let move_buffer = app.world.resource::<ClientSendBuffer<ObjectMoved>>();
        let move_event = move_buffer.iter().exactly_one().unwrap();
        assert_eq!(move_event.translation, TRANSFORM.translation);
        assert_eq!(move_event.rotation, TRANSFORM.rotation);
    }

    #[test]
    fn spawning_cursor_cancellation() {
        let mut app = App::new();
        app.init_resource::<ClientSendBuffer<PickCancelled>>()
            .add_plugin(TestCursorObjectPlugin);

        let cursor_entity = app
            .world
            .spawn()
            .insert(CursorObject::Spawning(PathBuf::new()))
            .id();

        // Wait for additional component insertion to avoid inserting and removing in the same frame.
        app.update();

        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::Cancel);

        app.update();

        assert!(app.world.get_entity(cursor_entity).is_none());
    }

    #[test]
    fn moving_cursor_cancellation() {
        let mut app = App::new();
        app.init_resource::<ClientSendBuffer<PickCancelled>>()
            .add_plugin(TestCursorObjectPlugin);

        let object_entity = app
            .world
            .spawn()
            .insert_bundle(DummyObjectBundle::default())
            .id();
        app.world
            .spawn()
            .insert(CursorObject::Moving(object_entity));

        app.update();

        let mut action_state = app.world.resource_mut::<ActionState<ControlAction>>();
        action_state.press(ControlAction::Cancel);

        app.update();

        let cancel_buffer = app.world.resource::<ClientSendBuffer<PickCancelled>>();
        assert_eq!(cancel_buffer.len(), 1);
    }

    #[test]
    fn spawning_cursor_despawn() {
        let mut app = App::new();
        app.init_resource::<ClientSendBuffer<PickCancelled>>()
            .add_plugin(TestCursorObjectPlugin);

        let cursor_entity = app
            .world
            .spawn()
            .insert(CursorObject::Spawning(PathBuf::new()))
            .id();

        app.world
            .resource_mut::<Events<SpawnConfirmed>>()
            .send(SpawnConfirmed);

        app.update();

        assert!(app.world.get_entity(cursor_entity).is_none());
    }

    #[test]
    fn moving_cursor_despawn() {
        let mut app = App::new();
        app.init_resource::<ClientSendBuffer<PickCancelled>>()
            .add_plugin(TestCursorObjectPlugin);

        let object_entity = app
            .world
            .spawn()
            .insert_bundle(DummyObjectBundle::default())
            .id();
        let cursor_entity = app
            .world
            .spawn()
            .insert(CursorObject::Moving(object_entity))
            .id();

        app.update();

        app.world
            .entity_mut(object_entity)
            .insert(Picked::default())
            .remove::<Picked>();

        app.update();

        let visibility = app.world.get::<Visibility>(object_entity).unwrap();
        assert!(visibility.is_visible);

        assert!(app.world.get_entity(cursor_entity).is_none());
    }

    #[test]
    fn object_spawning() {
        let mut app = App::new();
        app.add_plugin(TestCursorObjectPlugin);

        let city = app
            .world
            .spawn()
            .insert(City)
            .insert(Visibility::default())
            .id();

        const TRANSFORM: Transform =
            Transform::from_translation(Vec3::ONE).with_rotation(Quat::from_array([1.0; 4]));
        const CLIENT_ID: u64 = 1;
        const METADATA_PATH: &str = "dummy.toml";
        app.world
            .resource_mut::<Events<ClientEvent<ObjectSpawned>>>()
            .send(ClientEvent {
                client_id: CLIENT_ID,
                event: ObjectSpawned {
                    metadata_path: METADATA_PATH.into(),
                    translation: TRANSFORM.translation,
                    rotation: TRANSFORM.rotation,
                },
            });

        app.update();

        let (parent_sync, transform, object_path) = app
            .world
            .query::<(&ParentSync, &Transform, &ObjectPath)>()
            .single(&app.world);

        assert_eq!(parent_sync.0, city);
        assert_eq!(*transform, TRANSFORM);
        assert_eq!(&object_path.0, METADATA_PATH);

        let spawn_confirm_buffer = app.world.resource::<ServerSendBuffer<SpawnConfirmed>>();
        let confirm_event = spawn_confirm_buffer.iter().exactly_one().unwrap();
        assert!(
            matches!(confirm_event.mode, SendMode::Direct(client_id) if client_id == CLIENT_ID)
        );
    }

    #[test]
    fn object_moving() {
        let mut app = App::new();
        app.add_plugin(TestCursorObjectPlugin);

        const CLIENT_ID: u64 = 1;
        const TRANSFORM: Transform =
            Transform::from_translation(Vec3::ONE).with_rotation(Quat::from_array([1.0; 4]));
        let object_entity = app
            .world
            .spawn()
            .insert_bundle(DummyObjectBundle::default())
            .insert(Picked(CLIENT_ID))
            .id();
        app.world
            .spawn()
            .insert(CursorObject::Moving(object_entity));

        app.update();

        app.world
            .resource_mut::<Events<ClientEvent<ObjectMoved>>>()
            .send(ClientEvent {
                client_id: CLIENT_ID,
                event: ObjectMoved {
                    translation: TRANSFORM.translation,
                    rotation: TRANSFORM.rotation,
                },
            });

        app.update();

        let object_entity = app.world.entity(object_entity);
        assert!(object_entity.get::<Picked>().is_none());
        assert_eq!(*object_entity.get::<Transform>().unwrap(), TRANSFORM);
    }

    struct TestCursorObjectPlugin;

    impl Plugin for TestCursorObjectPlugin {
        fn build(&self, app: &mut App) {
            app.init_resource::<Windows>()
                .init_resource::<RapierContext>()
                .init_resource::<ActionState<ControlAction>>()
                .add_loopless_state(GameState::City)
                .add_plugin(CorePlugin)
                .add_plugin(AssetPlugin)
                .add_plugin(CursorObjectPlugin);
        }
    }

    #[derive(Bundle, Default, Clone)]
    struct DummyObjectBundle {
        transform: Transform,
        scene: Handle<Scene>,
        visibility: Visibility,
    }
}
