use std::{f32::consts::FRAC_PI_4, fmt::Debug};

use bevy::{asset::HandleId, math::Vec3Swizzles, prelude::*};
use bevy_rapier3d::prelude::*;
use bevy_scene_hook::{HookedSceneBundle, SceneHook};
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    asset_metadata::{self, ObjectMetadata},
    city::CityMode,
    collision_groups::DollisGroups,
    component_commands::ComponentCommandsExt,
    cursor_hover::CursorHover,
    family::FamilyMode,
    game_state::GameState,
    object::{ObjectDespawn, ObjectEventConfirmed, ObjectMove, ObjectPath, ObjectSpawn},
    player_camera::PlayerCamera,
    unique_asset::UniqueAsset,
};

#[derive(SystemLabel)]
enum PlacingObjectSystem {
    Rotation,
    Movement,
    Collision,
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
                .after(PlacingObjectSystem::Rotation)
                .label(PlacingObjectSystem::Movement),
        )
        .add_system(
            Self::collision_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects)
                .after(PlacingObjectSystem::Movement)
                .label(PlacingObjectSystem::Collision),
        )
        .add_system(
            Self::material_system
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects)
                .after(PlacingObjectSystem::Collision),
        )
        .add_system(
            Self::confirmation_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Objects)
                .after(PlacingObjectSystem::Collision),
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
                .run_if(action::just_pressed(Action::Confirm))
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
                .after(PlacingObjectSystem::Rotation)
                .label(PlacingObjectSystem::Movement),
        )
        .add_system(
            Self::collision_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .after(PlacingObjectSystem::Movement)
                .label(PlacingObjectSystem::Collision),
        )
        .add_system(
            Self::material_system
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .after(PlacingObjectSystem::Collision),
        )
        .add_system(
            Self::confirmation_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .after(PlacingObjectSystem::Collision),
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
        .add_exit_system(FamilyMode::Building, Self::cleanup_system)
        .add_system(Self::cleanup_system.run_on_event::<ObjectEventConfirmed>());
    }
}

impl PlacingObjectPlugin {
    fn picking_system(
        mut commands: Commands,
        hovered_objects: Query<(Entity, &Parent), (With<ObjectPath>, With<CursorHover>)>,
        children: Query<&Children>,
        mut groups: Query<&mut CollisionGroups>,
    ) {
        if let Ok((entity, parent)) = hovered_objects.get_single() {
            commands.entity(parent.get()).with_children(|parent| {
                parent.spawn(PlacingObject::moving(entity));
            });

            // To exclude from collision with the placing object.
            for child_entity in children.iter_descendants(entity) {
                if let Ok(mut group) = groups.get_mut(child_entity) {
                    group.memberships ^= Group::OBJECT;
                }
            }
        }
    }

    fn init_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        mut objects: Query<(&Transform, &Handle<Scene>, &ObjectPath, &mut Visibility)>,
        new_placing_objects: Query<(Entity, &PlacingObject), Added<PlacingObject>>,
    ) {
        for (placing_entity, placing_object) in &new_placing_objects {
            debug!("created placing object {placing_object:?}");

            let mut placing_entity = commands.entity(placing_entity);

            let (transform, scene_handle, object_metadata) = match placing_object.kind {
                PlacingObjectKind::Spawning(id) => {
                    let metadata_path = asset_server
                        .get_handle_path(id)
                        .expect("spawning object metadata should have a path");
                    let metadata_handle = asset_server.get_handle(id);
                    let object_metadata = object_metadata
                        .get(&metadata_handle)
                        .unwrap_or_else(|| panic!("object metadata {metadata_path:?} is invalid"));
                    let scene_handle =
                        asset_server.load(asset_metadata::scene_path(metadata_path.path()));
                    placing_entity.insert(CursorOffset::default());

                    (Transform::default(), scene_handle, object_metadata)
                }
                PlacingObjectKind::Moving(object_entity) => {
                    let (transform, scene_handle, object_path, mut visibility) = objects
                        .get_mut(object_entity)
                        .expect("moving object should exist with these components");
                    visibility.is_visible = false;
                    let metadata_handle = asset_server.load(&*object_path.0);
                    let object_metadata =
                        object_metadata.get(&metadata_handle).unwrap_or_else(|| {
                            panic!("path {:?} should correspond to metadata", object_path.0)
                        });

                    (*transform, scene_handle.clone(), object_metadata)
                }
            };

            placing_entity
                .insert((
                    AsyncSceneCollider::default(),
                    UniqueAsset::<StandardMaterial>::default(),
                    HookedSceneBundle {
                        scene: SceneBundle {
                            scene: scene_handle,
                            transform,
                            ..Default::default()
                        },
                        hook: SceneHook::new(|entity, commands| {
                            if entity.contains::<Handle<Mesh>>() {
                                commands.insert(CollisionGroups::new(Group::NONE, Group::NONE));
                            }
                        }),
                    },
                ))
                .insert_components(
                    object_metadata
                        .components
                        .iter()
                        .map(|component| component.clone_value())
                        .collect(),
                );
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
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
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
                        CollisionGroups::new(Group::ALL, Group::GROUND | Group::WALL).into(),
                    )
                    .map(|(_, toi)| toi)
                    .unwrap_or_default();

                let mut ray_translation = ray.origin + ray.direction * toi;
                ray_translation.y = 0.0;
                let offset = cursor_offset.copied().unwrap_or_else(|| {
                    let offset = CursorOffset(transform.translation.xz() - ray_translation.xz());
                    commands.entity(entity).insert(offset);
                    offset
                });
                transform.translation = ray_translation + Vec3::new(offset.x, 0.0, offset.y);
            }
        }
    }

    fn collision_system(
        rapier_ctx: Res<RapierContext>,
        mut placing_objects: Query<(Entity, &mut PlacingObject)>,
        children: Query<&Children>,
        child_meshes: Query<(&Collider, &GlobalTransform)>,
    ) {
        if let Ok((object_entity, mut placing_object)) = placing_objects.get_single_mut() {
            for (collider, transform) in children
                .iter_descendants(object_entity)
                .flat_map(|entity| child_meshes.get(entity))
            {
                let (_, rotation, translation) = transform.to_scale_rotation_translation();
                let mut intersects = false;
                rapier_ctx.intersections_with_shape(
                    translation,
                    rotation,
                    collider,
                    CollisionGroups::new(Group::ALL, Group::OBJECT | Group::WALL).into(),
                    |_| {
                        intersects = true;
                        false
                    },
                );
                if intersects {
                    if !placing_object.collides {
                        placing_object.collides = true;
                    }
                    return;
                }
            }

            if placing_object.collides {
                placing_object.collides = false;
            }
        }
    }

    fn material_system(
        mut materials: ResMut<Assets<StandardMaterial>>,
        placing_objects: Query<(Entity, &PlacingObject), Changed<PlacingObject>>,
        children: Query<&Children>,
        material_handles: Query<&Handle<StandardMaterial>>,
    ) {
        if let Ok((entity, placing_object)) = placing_objects.get_single() {
            for handle in children
                .iter_descendants(entity)
                .filter_map(|entity| material_handles.get(entity).ok())
            {
                let mut material = materials
                    .get_mut(handle)
                    .expect("material handle should be valid");
                material.base_color = if placing_object.collides {
                    Color::RED
                } else {
                    Color::WHITE
                };
            }
            debug!("assigned material color for {placing_object:?}");
        }
    }

    fn confirmation_system(
        mut move_events: EventWriter<ObjectMove>,
        mut spawn_events: EventWriter<ObjectSpawn>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(&Transform, &PlacingObject)>,
    ) {
        if let Ok((transform, placing_object)) = placing_objects.get_single() {
            if !placing_object.collides {
                debug!("confirmed placing object {placing_object:?}");
                match placing_object.kind {
                    PlacingObjectKind::Spawning(id) => {
                        let metadata_path = asset_server
                            .get_handle_path(id)
                            .expect("spawning object metadata should have a path");
                        spawn_events.send(ObjectSpawn {
                            metadata_path: metadata_path.path().to_path_buf(),
                            position: transform.translation.xz(),
                            rotation: transform.rotation,
                        });
                    }
                    PlacingObjectKind::Moving(entity) => move_events.send(ObjectMove {
                        entity,
                        translation: transform.translation,
                        rotation: transform.rotation,
                    }),
                }
            }
        }
    }

    fn despawn_system(
        mut commands: Commands,
        mut despawn_events: EventWriter<ObjectDespawn>,
        placing_objects: Query<(Entity, &PlacingObject)>,
    ) {
        if let Ok((entity, placing_object)) = placing_objects.get_single() {
            if let PlacingObjectKind::Moving(entity) = placing_object.kind {
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
        children: Query<&Children>,
        mut groups: Query<&mut CollisionGroups>,
    ) {
        if let Ok((placing_entity, placing_object)) = placing_objects.get_single() {
            debug!("despawned placing object {placing_object:?}");
            commands.entity(placing_entity).despawn_recursive();

            if let PlacingObjectKind::Moving(object_entity) = placing_object.kind {
                // Object could be invalid in case of removal.
                if let Ok(mut visibility) = visibility.get_mut(object_entity) {
                    visibility.is_visible = true;
                }

                // Restore object's collisions back.
                for child_entity in children.iter_descendants(object_entity) {
                    if let Ok(mut group) = groups.get_mut(child_entity) {
                        group.memberships |= Group::OBJECT;
                    }
                }
            }
        }
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct PlacingObject {
    kind: PlacingObjectKind,
    collides: bool,
}

impl PlacingObject {
    pub(crate) fn moving(object_entity: Entity) -> Self {
        Self {
            kind: PlacingObjectKind::Moving(object_entity),
            collides: false,
        }
    }

    pub(crate) fn spawning(id: HandleId) -> Self {
        Self {
            kind: PlacingObjectKind::Spawning(id),
            collides: false,
        }
    }

    pub(crate) fn spawning_id(&self) -> Option<HandleId> {
        match self.kind {
            PlacingObjectKind::Spawning(id) => Some(id),
            PlacingObjectKind::Moving(_) => None,
        }
    }
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Debug, Clone, Copy)]
pub(crate) enum PlacingObjectKind {
    Spawning(HandleId),
    Moving(Entity),
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
