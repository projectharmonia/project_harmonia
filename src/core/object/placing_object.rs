use std::{f32::consts::FRAC_PI_4, fmt::Debug, path::PathBuf};

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_renet::renet::RenetClient;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::*;

use super::{ObjectPickCancel, PickedPlayer};
use crate::core::{
    action::{self, Action},
    city::CityMode,
    collision_groups::DollisGroups,
    family::FamilyMode,
    game_state::GameState,
    network::server::SERVER_ID,
    object::{ObjectDespawn, ObjectMove, ObjectPath, ObjectSpawn, ObjectSpawnConfirmed},
    preview::PreviewCamera,
    suspend::{SuspendCommandsExt, Suspended},
};

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::picking_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_system(
            Self::movement_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_system(
            Self::confirmation_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Delete))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_system(
            Self::cancel_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_exit_system(CityMode::Objects, Self::cancel_system)
        .add_system(
            Self::spawning_cleanup_system
                .run_on_event::<ObjectSpawnConfirmed>()
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::movement_cleanup_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        // TODO 0.10: clone system set.
        .add_system(
            Self::picking_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system(
            Self::movement_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system(
            Self::confirmation_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Delete))
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system(
            Self::cancel_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_exit_system(FamilyMode::Building, Self::cancel_system)
        .add_system(
            Self::spawning_cleanup_system
                .run_on_event::<ObjectSpawnConfirmed>()
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::movement_cleanup_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        );
    }
}

impl PlacingObjectPlugin {
    fn picking_system(
        mut commands: Commands,
        client: Option<Res<RenetClient>>,
        new_picked_objects: Query<(Entity, &PickedPlayer), Added<PickedPlayer>>,
    ) {
        let client_id = client.map(|client| client.client_id()).unwrap_or(SERVER_ID);
        if let Some(entity) = new_picked_objects
            .iter()
            .find(|(_, picked_player)| picked_player.0 == client_id)
            .map(|(entity, _)| entity)
        {
            commands
                .entity(entity)
                .insert(PlacingObject::Moving)
                .suspend::<Transform>();
        }
    }

    fn movement_system(
        mut commands: Commands,
        windows: Res<Windows>,
        rapier_ctx: Res<RapierContext>,
        action_state: Res<ActionState<Action>>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
        mut placing_objects: Query<
            (Entity, &mut Transform, Option<&CursorOffset>),
            With<PlacingObject>,
        >,
    ) {
        if let Ok((entity, mut transform, cursor_offset)) = placing_objects.get_single_mut() {
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
                        CollisionGroups::new(Group::ALL, Group::GROUND).into(),
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
        mut move_events: EventWriter<ObjectMove>,
        mut spawn_events: EventWriter<ObjectSpawn>,
        placing_objects: Query<(&Transform, &PlacingObject, &ObjectPath)>,
    ) {
        if let Ok((transform, placing_object, object_path)) = placing_objects.get_single() {
            debug!("confirmed placing object {placing_object:?}");
            match placing_object {
                PlacingObject::Spawning => {
                    spawn_events.send(ObjectSpawn {
                        metadata_path: object_path.0.clone(),
                        position: transform.translation.xz(),
                        rotation: transform.rotation,
                    });
                }
                PlacingObject::Moving => move_events.send(ObjectMove {
                    translation: transform.translation,
                    rotation: transform.rotation,
                }),
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventWriter<ObjectDespawn>,
        placing_objects: Query<(Entity, &PlacingObject)>,
    ) {
        if let Ok((entity, placing_object)) = placing_objects.get_single() {
            match placing_object {
                PlacingObject::Spawning => {
                    debug!("sent despawn event for placing object {placing_object:?}");
                    despawn_events.send(ObjectDespawn);
                }
                PlacingObject::Moving => {
                    debug!("cancelled placing object {placing_object:?}");
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }

    fn cancel_system(
        mut commands: Commands,
        mut cancel_events: EventWriter<ObjectPickCancel>,
        placing_objects: Query<(Entity, &PlacingObject)>,
    ) {
        if let Ok((entity, placing_object)) = placing_objects.get_single() {
            match placing_object {
                PlacingObject::Spawning => commands.entity(entity).despawn_recursive(),
                PlacingObject::Moving => {
                    commands
                        .entity(entity)
                        .remove::<(PlacingObject, CursorOffset)>()
                        .restore_suspended::<Transform>();
                    cancel_events.send_default();
                }
            }
            debug!("cancelled placing object {placing_object:?}");
        }
    }

    fn movement_cleanup_system(
        mut commands: Commands,
        unpicked_objects: RemovedComponents<PickedPlayer>,
        placing_objects: Query<(), With<PlacingObject>>,
    ) {
        for entity in unpicked_objects.iter() {
            if placing_objects.get(entity).is_ok() {
                commands
                    .entity(entity)
                    .remove::<(PlacingObject, CursorOffset, Suspended<Transform>)>();
                debug!("despawned placing object");
            }
        }
    }

    fn spawning_cleanup_system(
        mut commands: Commands,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        commands
            .entity(placing_objects.single())
            .despawn_recursive();
        debug!("despawned placing object");
    }
}

pub(crate) fn placing_active(placing_objects: Query<(), With<PlacingObject>>) -> bool {
    !placing_objects.is_empty()
}

#[derive(Bundle)]
pub(crate) struct SpawningObjectBundle {
    object_path: ObjectPath,
    transform: Transform,
    cursor_offset: CursorOffset,
    placing_object: PlacingObject,
}

impl SpawningObjectBundle {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            object_path: ObjectPath(path),
            // Spawn at location invisible to the spawner, will be corrected later in movement system.
            transform: Transform::from_translation(Vec3::splat(f32::MAX)),
            cursor_offset: Default::default(),
            placing_object: PlacingObject::Spawning,
        }
    }
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) enum PlacingObject {
    Spawning,
    Moving,
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
