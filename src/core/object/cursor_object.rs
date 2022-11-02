use std::{f32::consts::FRAC_PI_4, fmt::Debug, path::PathBuf};

use bevy::{ecs::event::Event, math::Vec3Swizzles, prelude::*};
use bevy_mod_raycast::Ray3d;
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;
use serde::{Deserialize, Serialize};
use tap::{TapFallible, TapOptional};

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

use super::{ObjectBundle, PickCancel, PickDelete, Picked};

pub(super) struct CursorObjectPlugin;

impl Plugin for CursorObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_client_event::<ObjectMove>()
            .add_client_event::<ObjectSpawn>()
            .add_server_event::<CursorConfirm>()
            // Run in this stage to avoid visibility having effect earlier than spawning cursor object.
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::spawning_system.run_in_state(GameState::City),
            )
            .add_system(Self::movement_system.run_in_state(GameState::City))
            .add_system(
                Self::application_system
                    .run_in_state(GameState::City)
                    .run_if(confirm_pressed),
            )
            .add_system(
                Self::cancel_spawning_or_send_system::<PickCancel>
                    .run_in_state(GameState::City)
                    .run_if(cancel_pressed),
            )
            .add_system(
                Self::cancel_spawning_or_send_system::<PickDelete>
                    .run_in_state(GameState::City)
                    .run_if(delete_pressed),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::movement_confirmation_system.run_in_state(GameState::City),
            )
            .add_system(Self::despawn_system.run_on_event::<CursorConfirm>())
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
                            scene: asset_server.load(&asset_metadata::scene_path(metadata_path)),
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

    fn application_system(
        mut move_buffers: ResMut<ClientSendBuffer<ObjectMove>>,
        mut spawn_events: ResMut<ClientSendBuffer<ObjectSpawn>>,
        cursor_objects: Query<(&Transform, &CursorObject)>,
    ) {
        if let Ok((transform, cursor_object)) = cursor_objects.get_single() {
            debug!("confirmed cursor {cursor_object:?}");
            match cursor_object {
                CursorObject::Spawning(metadata_path) => spawn_events.push(ObjectSpawn {
                    metadata_path: metadata_path.clone(),
                    translation: transform.translation,
                    rotation: transform.rotation,
                }),
                CursorObject::Moving(_) => move_buffers.push(ObjectMove {
                    translation: transform.translation,
                    rotation: transform.rotation,
                }),
            }
        }
    }

    fn cancel_spawning_or_send_system<T: Event + Default + Debug>(
        mut commands: Commands,
        mut client_buffer: ResMut<ClientSendBuffer<T>>,
        moving_objects: Query<(Entity, &CursorObject)>,
    ) {
        if let Ok((entity, cursor_object)) = moving_objects.get_single() {
            if let CursorObject::Spawning(_) = cursor_object {
                debug!("cancelled cursor {cursor_object:?}");
                commands.entity(entity).despawn_recursive();
            } else {
                let event = T::default();
                debug!("sent event {event:?} for cursor {cursor_object:?}");
                client_buffer.push(event);
            }
        }
    }

    fn movement_confirmation_system(
        pick_removals: RemovedComponents<Picked>,
        mut confirm_events: EventWriter<CursorConfirm>,
        cursor_objects: Query<&CursorObject>,
    ) {
        if let Ok(CursorObject::Moving(object_entity)) = cursor_objects.get_single() {
            if pick_removals
                .iter()
                .any(|unpicked_entity| unpicked_entity == *object_entity)
            {
                debug!("movement was confirmed for {object_entity:?}");
                confirm_events.send_default();
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        cursor_objects: Query<(Entity, &CursorObject)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        if let Ok((cursor_entity, cursor_object)) = cursor_objects
            .get_single()
            .tap_err(|e| error!("unable to get cursor object for despawn: {e}"))
        {
            debug!("despawned cursor {cursor_object:?}");
            match *cursor_object {
                CursorObject::Spawning(_) => {
                    commands.entity(cursor_entity).despawn_recursive();
                }
                CursorObject::Moving(object_entity) => {
                    commands.entity(cursor_entity).despawn_recursive();
                    // Object could be invalid in case of removal.
                    if let Ok(mut visibility) = visibility.get_mut(object_entity) {
                        visibility.is_visible = true;
                    }
                }
            }
        }
    }

    fn spawn_object_system(
        mut commands: Commands,
        mut spawn_events: EventReader<ClientEvent<ObjectSpawn>>,
        mut confirm_buffer: ResMut<ServerSendBuffer<CursorConfirm>>,
        visible_cities: Query<Entity, (With<City>, With<Visibility>)>,
    ) {
        for ClientEvent { client_id, event } in spawn_events.iter().cloned() {
            commands.spawn_bundle(ObjectBundle::new(
                event.metadata_path,
                event.translation,
                event.rotation,
                visible_cities.single(),
            ));
            confirm_buffer.push(ServerEvent {
                mode: SendMode::Direct(client_id),
                event: CursorConfirm,
            });
        }
    }

    fn apply_movement_system(
        mut commands: Commands,
        mut move_events: EventReader<ClientEvent<ObjectMove>>,
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

fn cancel_pressed(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.just_pressed(ControlAction::Cancel)
}

fn confirm_pressed(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.just_pressed(ControlAction::Confirm)
}

fn delete_pressed(action_state: Res<ActionState<ControlAction>>) -> bool {
    action_state.just_pressed(ControlAction::Delete)
}

pub(crate) fn cursor_object_exists(cursor_objects: Query<(), With<CursorObject>>) -> bool {
    !cursor_objects.is_empty()
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ObjectMove {
    translation: Vec3,
    rotation: Quat,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ObjectSpawn {
    metadata_path: PathBuf,
    translation: Vec3,
    rotation: Quat,
}

/// An event for cursor action confirmation.
///
/// Could be received as a network event from server for spawning cursor or
/// emitted locally when the received server state contains pick removal for moving cursor.
#[derive(Deserialize, Serialize, Debug, Default)]
struct CursorConfirm;

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug)]
pub(crate) enum CursorObject {
    Spawning(PathBuf),
    Moving(Entity),
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
