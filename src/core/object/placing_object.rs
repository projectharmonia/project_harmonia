use std::{
    f32::consts::{FRAC_PI_4, PI},
    fmt::Debug,
};

use bevy::{
    ecs::{reflect::ReflectCommandExt, system::EntityCommands},
    math::Vec3Swizzles,
    prelude::*,
    scene::{self, SceneInstanceReady},
    window::PrimaryWindow,
};
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::core::{
    action::Action,
    asset::metadata::{self, object_metadata::ObjectMetadata},
    city::CityMode,
    cursor_hover::{CursorHover, CursorHoverSettings},
    family::FamilyMode,
    game_state::GameState,
    object::{ObjectDespawn, ObjectEventConfirmed, ObjectMove, ObjectPath, ObjectSpawn},
    player_camera::PlayerCamera,
    wall::wall_object::WallObject,
    Layer,
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub(crate) struct ObjectSnappingSet;

pub(crate) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            ObjectSnappingSet
                .after(Self::movement_system)
                .before(Self::collision_system)
                .run_if(
                    in_state(GameState::City)
                        .and_then(in_state(CityMode::Objects))
                        .or_else(
                            in_state(GameState::Family).and_then(in_state(FamilyMode::Building)),
                        ),
                ),
        )
        .add_systems(OnExit(CityMode::Objects), Self::cancel_system)
        .add_systems(OnExit(FamilyMode::Building), Self::cancel_system)
        .add_systems(
            Update,
            (
                (
                    Self::init_system,
                    Self::picking_system
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<PlacingObject>())),
                    Self::confirmation_system
                        .after(Self::collision_system)
                        .run_if(action_just_pressed(Action::Confirm)),
                    Self::despawn_system.run_if(action_just_pressed(Action::Delete)),
                    Self::cancel_system.run_if(
                        action_just_pressed(Action::Cancel)
                            .or_else(on_event::<ObjectEventConfirmed>()),
                    ),
                ),
                (
                    Self::rotation_system.run_if(action_just_pressed(Action::RotateObject)),
                    Self::movement_system,
                    Self::collision_system,
                    Self::material_system,
                )
                    .chain(),
            )
                .run_if(
                    in_state(GameState::City)
                        .and_then(in_state(CityMode::Objects))
                        .or_else(
                            in_state(GameState::Family).and_then(in_state(FamilyMode::Building)),
                        ),
                ),
        )
        .add_systems(
            SpawnScene,
            Self::scene_init_system
                .run_if(
                    in_state(GameState::City)
                        .and_then(in_state(CityMode::Objects))
                        .or_else(
                            in_state(GameState::Family).and_then(in_state(FamilyMode::Building)),
                        ),
                )
                .after(scene::scene_spawner_system),
        )
        .add_systems(PostUpdate, Self::exclusive_system);
    }
}

impl PlacingObjectPlugin {
    fn picking_system(
        mut commands: Commands,
        hovered_objects: Query<(Entity, &Parent), (With<ObjectPath>, With<CursorHover>)>,
    ) {
        if let Ok((placing_entity, parent)) = hovered_objects.get_single() {
            commands.entity(**parent).with_children(|parent| {
                parent.spawn(PlacingObject::moving(placing_entity));
            });
        }
    }

    fn init_system(
        mut commands: Commands,
        mut hover_settings: ResMut<CursorHoverSettings>,
        asset_server: Res<AssetServer>,
        object_metadata: Res<Assets<ObjectMetadata>>,
        objects: Query<(&Transform, &Handle<Scene>, &ObjectPath)>,
        new_placing_objects: Query<(Entity, &PlacingObject), Added<PlacingObject>>,
    ) {
        for (placing_entity, placing_object) in &new_placing_objects {
            debug!("created placing object {placing_object:?}");

            let mut placing_entity = commands.entity(placing_entity);

            let (transform, scene_handle, metadata) = match placing_object.kind {
                PlacingObjectKind::Spawning(metadata_id) => {
                    let scene_handle =
                        asset_server.load(metadata::scene_path(&asset_server, metadata_id));
                    let metadata = object_metadata.get(metadata_id).unwrap();
                    placing_entity.insert(CursorOffset::default());
                    let transform = Transform::from_rotation(Quat::from_rotation_y(PI)); // Rotate towards camera.

                    (transform, scene_handle, metadata)
                }
                PlacingObjectKind::Moving(object_entity) => {
                    let (transform, scene_handle, object_path) = objects
                        .get(object_entity)
                        .expect("moving object should exist with these components");
                    let metadata_handle = asset_server.load(&object_path.0);
                    let metadata = object_metadata.get(&metadata_handle).unwrap();

                    (*transform, scene_handle.clone(), metadata)
                }
            };

            placing_entity.insert((
                Name::new("Placing object"),
                SceneBundle {
                    scene: scene_handle,
                    transform,
                    ..Default::default()
                },
            ));
            for component in &metadata.components {
                placing_entity.insert_reflect(component.clone_value());
            }

            hover_settings.enabled = false;
        }
    }

    fn scene_init_system(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        placing_objects: Query<(Entity, &PlacingObject)>,
        chidlren: Query<&Children>,
        mut objects: Query<&mut Visibility>,
        mesh_handles: Query<(Entity, &Handle<Mesh>)>,
    ) {
        for (object_entity, placing_object) in
            placing_objects.iter_many(ready_events.read().map(|event| event.parent))
        {
            if let PlacingObjectKind::Moving(object_entity) = placing_object.kind {
                let mut visibility = objects
                    .get_mut(object_entity)
                    .expect("moving object reference a valid object");
                *visibility = Visibility::Hidden;
            }

            for (child_entity, mesh_handle) in
                mesh_handles.iter_many(chidlren.iter_descendants(object_entity))
            {
                if let Some(mesh) = meshes.get(mesh_handle) {
                    if let Some(collider) = Collider::trimesh_from_mesh(mesh) {
                        commands
                            .entity(child_entity)
                            .insert((collider, CollisionLayers::none()));
                    }
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
        spatial_query: SpatialQuery,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
        mut placing_objects: Query<(
            Entity,
            &mut Transform,
            &PlacingObject,
            Option<&CursorOffset>,
        )>,
    ) {
        let Ok((entity, mut transform, placing_object, cursor_offset)) =
            placing_objects.get_single_mut()
        else {
            return;
        };
        let Some(cursor_pos) = windows.single().cursor_position() else {
            return;
        };

        let (&camera_transform, camera) = cameras.single();
        let ray = camera
            .viewport_to_world(&camera_transform, cursor_pos)
            .expect("ray should be created from screen coordinates");

        let mut filter = SpatialQueryFilter::new().with_masks([Layer::Ground, Layer::Wall]);
        if let PlacingObjectKind::Moving(entity) = placing_object.kind {
            filter.excluded_entities.insert(entity);
        }

        let Some(hit) = spatial_query.cast_ray(ray.origin, ray.direction, f32::MAX, false, filter)
        else {
            return;
        };

        let mut ray_translation = ray.origin + ray.direction * hit.time_of_impact;
        ray_translation.y = 0.0;
        let offset = cursor_offset.copied().unwrap_or_else(|| {
            let offset = CursorOffset(transform.translation.xz() - ray_translation.xz());
            commands.entity(entity).insert(offset);
            offset
        });
        transform.translation = ray_translation + Vec3::new(offset.x, 0.0, offset.y);
    }

    fn collision_system(
        spatial_query: SpatialQuery,
        mut placing_objects: Query<(Entity, &mut PlacingObject, &WallObject)>,
        children: Query<&Children>,
        child_meshes: Query<(&Collider, &GlobalTransform)>,
    ) {
        let Ok((object_entity, mut placing_object, &wall_object)) =
            placing_objects.get_single_mut()
        else {
            return;
        };

        let mut filter = SpatialQueryFilter::new().with_masks([Layer::Object]);
        if wall_object == WallObject::Fixture {
            filter.masks |= Layer::Wall.to_bits();
        };

        for (collider, transform) in
            child_meshes.iter_many(children.iter_descendants(object_entity))
        {
            let (_, rotation, translation) = transform.to_scale_rotation_translation();
            if !spatial_query
                .shape_intersections(collider, translation, rotation, filter.clone())
                .is_empty()
            {
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

    fn material_system(
        mut materials: ResMut<Assets<StandardMaterial>>,
        placing_objects: Query<(Entity, &PlacingObject), Changed<PlacingObject>>,
        children: Query<&Children>,
        mut material_handles: Query<&mut Handle<StandardMaterial>>,
    ) {
        if let Ok((placing_entity, placing_object)) = placing_objects.get_single() {
            let mut iter =
                material_handles.iter_many_mut(children.iter_descendants(placing_entity));
            while let Some(mut material_handle) = iter.fetch_next() {
                let mut material = materials
                    .get(&*material_handle)
                    .cloned()
                    .expect("material handle should be valid");

                material.base_color = if placing_object.collides || !placing_object.allowed_place {
                    Color::RED
                } else {
                    Color::WHITE
                };
                *material_handle = materials.add(material);
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
            if !placing_object.collides && placing_object.allowed_place {
                debug!("confirmed placing object {placing_object:?}");
                match placing_object.kind {
                    PlacingObjectKind::Spawning(metadata_id) => {
                        let metadata_path = asset_server
                            .get_path(metadata_id)
                            .expect("metadata should always come from file");
                        spawn_events.send(ObjectSpawn {
                            metadata_path: metadata_path.into_owned(),
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

    fn exclusive_system(
        mut commands: Commands,
        new_placing_objects: Query<Entity, Added<PlacingObject>>,
        placing_objects: Query<(Entity, &PlacingObject)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        if let Some(new_entity) = new_placing_objects.iter().last() {
            for (placing_entity, placing_object) in &placing_objects {
                if placing_entity != new_entity {
                    cleanup(
                        commands.entity(placing_entity),
                        placing_object.kind,
                        &mut visibility,
                    );
                }
            }
        }
    }

    fn cancel_system(
        mut commands: Commands,
        mut hover_settings: ResMut<CursorHoverSettings>,
        placing_objects: Query<(Entity, &PlacingObject)>,
        mut visibility: Query<&mut Visibility>,
    ) {
        hover_settings.enabled = true;

        for (placing_entity, placing_object) in &placing_objects {
            cleanup(
                commands.entity(placing_entity),
                placing_object.kind,
                &mut visibility,
            );
        }
    }
}

fn cleanup(
    placing_entity: EntityCommands,
    kind: PlacingObjectKind,
    visibility: &mut Query<&mut Visibility>,
) {
    debug!("despawned placing object {kind:?}");
    placing_entity.despawn_recursive();

    if let PlacingObjectKind::Moving(object_entity) = kind {
        // Object could be invalid in case of removal.
        if let Ok(mut visibility) = visibility.get_mut(object_entity) {
            *visibility = Visibility::Visible;
        }
    }
}

#[derive(Component, Debug, Clone)]
pub(crate) struct PlacingObject {
    kind: PlacingObjectKind,
    collides: bool,
    pub(crate) allowed_place: bool,
}

impl PlacingObject {
    pub(crate) fn moving(object_entity: Entity) -> Self {
        Self {
            kind: PlacingObjectKind::Moving(object_entity),
            collides: false,
            allowed_place: true,
        }
    }

    pub(crate) fn spawning(metadata_id: AssetId<ObjectMetadata>) -> Self {
        Self {
            kind: PlacingObjectKind::Spawning(metadata_id),
            collides: false,
            allowed_place: true,
        }
    }
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Clone, Copy, Debug)]
pub(crate) enum PlacingObjectKind {
    Spawning(AssetId<ObjectMetadata>),
    Moving(Entity),
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec2);
