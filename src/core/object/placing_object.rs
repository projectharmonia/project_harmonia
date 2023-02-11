use std::{f32::consts::FRAC_PI_4, fmt::Debug};

use bevy::{asset::HandleId, math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    asset_metadata,
    city::CityMode,
    collision_groups::DollisGroups,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    object::{ObjectDespawn, ObjectEventConfirmed, ObjectMove, ObjectPath, ObjectSpawn},
    preview::PreviewCamera,
};

#[derive(SystemLabel)]
enum PlacingObjectSystem {
    Rotation,
}

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::picking_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        // Run in this stage to avoid visibility having effect earlier than spawning placing object.
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::init_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_system(
            Self::rotation_system
                .run_if(action::just_pressed(Action::RotateObject))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects)
                .label(PlacingObjectSystem::Rotation),
        )
        .add_system(
            Self::movement_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects)
                .after(PlacingObjectSystem::Rotation),
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
            Self::cleanup_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects),
        )
        .add_exit_system(CityMode::Objects, Self::cleanup_system)
        // TODO 0.10: clone system set.
        .add_system(
            Self::picking_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        // Run in this stage to avoid visibility having effect earlier than spawning placing object.
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::init_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system(
            Self::rotation_system
                .run_if(action::just_pressed(Action::RotateObject))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects)
                .label(PlacingObjectSystem::Rotation),
        )
        .add_system(
            Self::movement_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .after(PlacingObjectSystem::Rotation),
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
            Self::cleanup_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building),
        )
        .add_system(Self::cleanup_system.run_on_event::<ObjectEventConfirmed>())
        .add_exit_system(FamilyMode::Building, Self::cleanup_system);
    }
}

impl PlacingObjectPlugin {
    fn picking_system(
        mut commands: Commands,
        hovered_objects: Query<(Entity, &Parent), (With<ObjectPath>, With<CursorHover>)>,
    ) {
        if let Ok((entity, parent)) = hovered_objects.get_single() {
            commands.entity(parent.get()).with_children(|parent| {
                parent.spawn(PlacingObject::Moving(entity));
            });
        }
    }

    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut objects: Query<(&Transform, &Handle<Scene>, &mut Visibility)>,
        new_placing_objects: Query<(Entity, &PlacingObject), Added<PlacingObject>>,
    ) {
        for (placing_entity, placing_object) in &new_placing_objects {
            debug!("created placing object {placing_object:?}");
            match *placing_object {
                PlacingObject::Spawning(id) => {
                    let metadata_path = asset_server
                        .get_handle_path(id)
                        .expect("spawning object metadata should have a path");
                    commands.entity(placing_entity).insert((
                        SceneBundle {
                            scene: asset_server
                                .load(asset_metadata::scene_path(metadata_path.path())),
                            ..Default::default()
                        },
                        CursorOffset::default(),
                    ));
                }
                PlacingObject::Moving(object_entity) => {
                    let (transform, scene_handle, mut visibility) = objects
                        .get_mut(object_entity)
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

    fn rotation_system(mut placing_objects: Query<&mut Transform, With<PlacingObject>>) {
        if let Ok(mut transform) = placing_objects.get_single_mut() {
            const ROTATION_STEP: f32 = -FRAC_PI_4;
            transform.rotate_y(ROTATION_STEP);
        }
    }

    fn movement_system(
        mut commands: Commands,
        windows: Res<Windows>,
        rapier_ctx: Res<RapierContext>,
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
            }
        }
    }

    fn confirmation_system(
        mut move_events: EventWriter<ObjectMove>,
        mut spawn_events: EventWriter<ObjectSpawn>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(&Transform, &PlacingObject)>,
    ) {
        if let Ok((transform, placing_object)) = placing_objects.get_single() {
            debug!("confirmed placing object {placing_object:?}");
            match *placing_object {
                PlacingObject::Spawning(id) => {
                    let metadata_path = asset_server
                        .get_handle_path(id)
                        .expect("spawning object metadata should have a path");
                    spawn_events.send(ObjectSpawn {
                        metadata_path: metadata_path.path().to_path_buf(),
                        position: transform.translation.xz(),
                        rotation: transform.rotation,
                    });
                }
                PlacingObject::Moving(entity) => move_events.send(ObjectMove {
                    entity,
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
            if let PlacingObject::Moving(entity) = *placing_object {
                debug!("sent despawn event for placing object {placing_object:?}");
                despawn_events.send(ObjectDespawn(entity));
            } else {
                debug!("cancelled placing object {placing_object:?}");
                commands.entity(entity).despawn_recursive();
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        placing_objects: Query<(Entity, &PlacingObject)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        if let Ok((placing_entity, placing_object)) = placing_objects.get_single() {
            debug!("despawned placing object {placing_object:?}");
            commands.entity(placing_entity).despawn_recursive();

            if let PlacingObject::Moving(object_entity) = *placing_object {
                // Object could be invalid in case of removal.
                if let Ok(mut visibility) = visibility.get_mut(object_entity) {
                    visibility.is_visible = true;
                }
            }
        }
    }
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug, Clone, Copy)]
pub(crate) enum PlacingObject {
    Spawning(HandleId),
    Moving(Entity),
}

impl PlacingObject {
    pub(crate) fn spawning_id(&self) -> Option<HandleId> {
        match self {
            PlacingObject::Spawning(id) => Some(*id),
            PlacingObject::Moving(_) => None,
        }
    }
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
