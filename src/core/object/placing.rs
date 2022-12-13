use std::{f32::consts::FRAC_PI_4, fmt::Debug, path::PathBuf};

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;

use crate::core::{
    action::{self, Action},
    asset_metadata,
    city::ActiveCity,
    game_state::{CursorMode, GameState},
    network::network_event::client_event::ClientSendBuffer,
    object::{ObjectConfirmed, ObjectDespawn, ObjectMove, ObjectPath, ObjectSpawn},
    picking::ObjectPicked,
    preview::PreviewCamera,
};

pub(super) struct PlacingPlugin;

impl Plugin for PlacingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::picking_system
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Objects),
        )
        // Run in this stage to avoid visibility having effect earlier than spawning placing object.
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::init_system
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Objects),
        )
        .add_system(
            Self::movement_system
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Objects),
        )
        .add_system(
            Self::confirmation_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Objects),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Delete))
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Objects),
        )
        .add_system(
            Self::cleanup_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::City)
                .run_in_state(CursorMode::Objects),
        )
        .add_system(Self::cleanup_system.run_on_event::<ObjectConfirmed>())
        .add_exit_system(CursorMode::Objects, Self::cleanup_system);
    }
}

impl PlacingPlugin {
    fn picking_system(
        mut commands: Commands,
        mut pick_events: EventReader<ObjectPicked>,
        parents: Query<&Parent, With<ObjectPath>>,
    ) {
        for event in pick_events.iter() {
            if let Ok(parent) = parents.get(event.entity) {
                commands.entity(parent.get()).with_children(|parent| {
                    parent.spawn(Placing::Moving(event.entity));
                });
            }
        }
    }

    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut objects: Query<(&Transform, &Handle<Scene>, &mut Visibility)>,
        new_placings: Query<(Entity, &Placing), Added<Placing>>,
    ) {
        for (placing_entity, placing) in &new_placings {
            debug!("started placing {placing:?}");
            match placing {
                Placing::Spawning(metadata_path) => {
                    commands.entity(placing_entity).insert((
                        SceneBundle {
                            scene: asset_server.load(&asset_metadata::scene_path(metadata_path)),
                            ..Default::default()
                        },
                        CursorOffset::default(),
                    ));
                }
                Placing::Moving(object_entity) => {
                    let (transform, scene_handle, mut visibility) = objects
                        .get_mut(*object_entity)
                        .expect("moving object should exist with these components");
                    commands.entity(placing_entity).insert(SceneBundle {
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
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
        mut placings: Query<(Entity, &mut Transform, Option<&CursorOffset>), With<Placing>>,
    ) {
        if let Ok((entity, mut transform, cursor_offset)) = placings.get_single_mut() {
            if let Some(cursor_pos) = windows
                .get_primary()
                .and_then(|window| window.cursor_position())
            {
                let (&camera_transform, camera) = cameras.single();
                let ray = camera
                    .viewport_to_world(&camera_transform, cursor_pos)
                    .expect("ray should be created from screen coordinates");

                let toi = rapier_ctx
                    .cast_ray(
                        ray.origin,
                        ray.direction,
                        f32::MAX,
                        false,
                        QueryFilter::new(),
                    )
                    .map(|(_, toi)| toi)
                    .unwrap_or_default();

                let ray_translation = ray.origin + ray.direction * toi;
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

    fn confirmation_system(
        mut move_buffers: ResMut<ClientSendBuffer<ObjectMove>>,
        mut spawn_events: ResMut<ClientSendBuffer<ObjectSpawn>>,
        placings: Query<(&Transform, &Placing)>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Ok((transform, placing)) = placings.get_single() {
            debug!("confirmed placing {placing:?}");
            match placing {
                Placing::Spawning(metadata_path) => spawn_events.push(ObjectSpawn {
                    metadata_path: metadata_path.clone(),
                    translation: transform.translation,
                    rotation: transform.rotation,
                    city_entity: active_cities.single(),
                }),
                Placing::Moving(entity) => move_buffers.push(ObjectMove {
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
        placings: Query<(Entity, &Placing)>,
    ) {
        if let Ok((entity, placing)) = placings.get_single() {
            if let Placing::Moving(entity) = *placing {
                debug!("sent despawn event for placing {placing:?}");
                despawn_buffer.push(ObjectDespawn(entity));
            } else {
                debug!("cancelled placing {placing:?}");
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        placings: Query<(Entity, &Placing)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        if let Ok((placing_entity, placing)) = placings.get_single() {
            debug!("despawned placing {placing:?}");
            commands.entity(placing_entity).despawn_recursive();

            if let Placing::Moving(object_entity) = *placing {
                // Object could be invalid in case of removal.
                if let Ok(mut visibility) = visibility.get_mut(object_entity) {
                    visibility.is_visible = true;
                }
            }
        }
    }
}

pub(crate) fn placing_active(placings: Query<(), With<Placing>>) -> bool {
    !placings.is_empty()
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug)]
pub(crate) enum Placing {
    Spawning(PathBuf),
    Moving(Entity),
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
