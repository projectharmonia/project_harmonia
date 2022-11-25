use std::{f32::consts::FRAC_PI_4, fmt::Debug, path::PathBuf};

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_mod_raycast::Ray3d;
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::core::{
    action::{self, Action},
    asset_metadata,
    city::ActiveCity,
    game_state::GameState,
    network::network_event::client_event::ClientSendBuffer,
    picking::ObjectPicked,
    preview::PreviewCamera,
};

use super::{ObjectConfirmed, ObjectDespawn, ObjectMove, ObjectPath, ObjectSpawn};

pub(super) struct CursorObjectPlugin;

impl Plugin for CursorObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::picking_system.run_in_state(GameState::City))
            // Run in this stage to avoid visibility having effect earlier than spawning cursor object.
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::init_system.run_in_state(GameState::City),
            )
            .add_system(Self::movement_system.run_in_state(GameState::City))
            .add_system(
                Self::application_system
                    .run_if(action::just_pressed(Action::Confirm))
                    .run_in_state(GameState::City),
            )
            .add_system(
                Self::despawn_system
                    .run_if(action::just_pressed(Action::Delete))
                    .run_in_state(GameState::City),
            )
            .add_system(
                Self::cleanup_system
                    .run_if(action::just_pressed(Action::Cancel))
                    .run_in_state(GameState::City),
            )
            .add_system(Self::cleanup_system.run_on_event::<ObjectConfirmed>());
    }
}

impl CursorObjectPlugin {
    fn picking_system(
        mut commands: Commands,
        mut pick_events: EventReader<ObjectPicked>,
        parents: Query<&Parent, With<ObjectPath>>,
    ) {
        for event in pick_events.iter() {
            if let Ok(parent) = parents.get(event.entity) {
                commands.entity(parent.get()).with_children(|parent| {
                    parent.spawn(CursorObject::Moving(event.entity));
                });
            }
        }
    }

    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut objects: Query<(&Transform, &Handle<Scene>, &mut Visibility)>,
        new_cursor_objects: Query<(Entity, &CursorObject), Added<CursorObject>>,
    ) {
        for (cursor_entity, cursor_object) in &new_cursor_objects {
            debug!("created cursor {cursor_object:?}");
            match cursor_object {
                CursorObject::Spawning(metadata_path) => {
                    commands.entity(cursor_entity).insert((
                        SceneBundle {
                            scene: asset_server.load(&asset_metadata::scene_path(metadata_path)),
                            ..Default::default()
                        },
                        CursorOffset::default(),
                    ));
                }
                CursorObject::Moving(object_entity) => {
                    let (transform, scene_handle, mut visibility) = objects
                        .get_mut(*object_entity)
                        .expect("moving object should exist with these components");
                    commands.entity(cursor_entity).insert(SceneBundle {
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
        action_state: Res<ActionState<Action>>,
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
                if action_state.just_pressed(Action::RotateObject) {
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
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Ok((transform, cursor_object)) = cursor_objects.get_single() {
            debug!("confirmed cursor {cursor_object:?}");
            match cursor_object {
                CursorObject::Spawning(metadata_path) => spawn_events.push(ObjectSpawn {
                    metadata_path: metadata_path.clone(),
                    translation: transform.translation,
                    rotation: transform.rotation,
                    city_entity: active_cities.single(),
                }),
                CursorObject::Moving(entity) => move_buffers.push(ObjectMove {
                    entity: *entity,
                    translation: transform.translation,
                    rotation: transform.rotation,
                }),
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_buffer: ResMut<ClientSendBuffer<ObjectDespawn>>,
        cursor_objects: Query<(Entity, &CursorObject)>,
    ) {
        if let Ok((entity, cursor_object)) = cursor_objects.get_single() {
            if let CursorObject::Moving(entity) = *cursor_object {
                debug!("sent despawn event for cursor {cursor_object:?}");
                despawn_buffer.push(ObjectDespawn(entity));
            } else {
                debug!("cancelled cursor {cursor_object:?}");
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        cursor_objects: Query<(Entity, &CursorObject)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        if let Ok((cursor_entity, cursor_object)) = cursor_objects.get_single() {
            debug!("despawned cursor {cursor_object:?}");
            commands.entity(cursor_entity).despawn_recursive();

            if let CursorObject::Moving(object_entity) = *cursor_object {
                // Object could be invalid in case of removal.
                if let Ok(mut visibility) = visibility.get_mut(object_entity) {
                    visibility.is_visible = true;
                }
            }
        }
    }
}

pub(crate) fn cursor_object_exists(cursor_objects: Query<(), With<CursorObject>>) -> bool {
    !cursor_objects.is_empty()
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug)]
pub(crate) enum CursorObject {
    Spawning(PathBuf),
    Moving(Entity),
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
