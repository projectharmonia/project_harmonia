use std::{
    f32::consts::{FRAC_PI_4, PI},
    fmt::Debug,
};

use bevy::{math::Vec3Swizzles, prelude::*, window::PrimaryWindow};
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::core::{
    action::Action,
    asset::metadata::object_metadata::ObjectMetadata,
    city::CityMode,
    cursor_hover::{CursorHover, CursorHoverSettings},
    family::FamilyMode,
    game_state::GameState,
    object::{ObjectBuy, ObjectEventConfirmed, ObjectMove, ObjectPath, ObjectSell},
    player_camera::PlayerCamera,
    Layer,
};

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(CityMode::Objects), Self::cancel)
            .add_systems(OnExit(FamilyMode::Building), Self::cancel)
            .add_systems(
                Update,
                (
                    (
                        Self::pick
                            .run_if(action_just_pressed(Action::Confirm))
                            .run_if(not(any_with_component::<PlacingObject>)),
                        Self::confirm
                            .after(Self::check_collision)
                            .run_if(action_just_pressed(Action::Confirm)),
                        Self::sell.run_if(action_just_pressed(Action::Delete)),
                        Self::cancel.run_if(
                            action_just_pressed(Action::Cancel)
                                .or_else(on_event::<ObjectEventConfirmed>()),
                        ),
                    ),
                    (
                        Self::rotate.run_if(action_just_pressed(Action::RotateObject)),
                        Self::apply_transform,
                        Self::check_collision,
                    )
                        .chain(),
                )
                    .run_if(
                        in_state(GameState::City)
                            .and_then(in_state(CityMode::Objects))
                            .or_else(
                                in_state(GameState::Family)
                                    .and_then(in_state(FamilyMode::Building)),
                            ),
                    ),
            )
            .add_systems(
                PostUpdate,
                (Self::update_materials, Self::init, Self::ensure_single).run_if(
                    in_state(GameState::City)
                        .and_then(in_state(CityMode::Objects))
                        .or_else(
                            in_state(GameState::Family).and_then(in_state(FamilyMode::Building)),
                        ),
                ),
            );
    }
}

impl PlacingObjectPlugin {
    fn pick(
        mut commands: Commands,
        hovered_objects: Query<(Entity, &Parent), (With<ObjectPath>, With<CursorHover>)>,
    ) {
        if let Ok((placing_entity, parent)) = hovered_objects.get_single() {
            commands.entity(**parent).with_children(|parent| {
                parent.spawn(PlacingObject::moving(placing_entity));
            });
        }
    }

    /// Inserts necessary components to trigger object initialization.
    fn init(
        mut commands: Commands,
        mut hover_settings: ResMut<CursorHoverSettings>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(Entity, &PlacingObject), Added<PlacingObject>>,
        objects: Query<(&Transform, &ObjectPath)>,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
    ) {
        let Some((placing_entity, placing_object)) = placing_objects.iter().last() else {
            return;
        };

        debug!("creating {placing_object:?}");
        match placing_object.kind {
            PlacingObjectKind::Spawning(id) => {
                let metadata_path = asset_server
                    .get_path(id)
                    .expect("metadata should always come from file");

                commands.entity(placing_entity).insert((
                    ObjectPath(metadata_path.into_owned()),
                    CursorOffset::default(),
                    Transform::from_rotation(Quat::from_rotation_y(PI)), // Rotate towards camera.
                ));
            }
            PlacingObjectKind::Moving(object_entity) => {
                let (&object_transform, object_path) = objects
                    .get(object_entity)
                    .expect("moving object should have scene and path");

                let (&camera_transform, camera) = cameras.single();
                let cursor_pos = windows.single().cursor_position().unwrap_or_default();
                let ray = camera
                    .viewport_to_world(&camera_transform, cursor_pos)
                    .expect("camera should always have a viewport");
                let distance = ray
                    .intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y))
                    .expect("camera should always look at the ground");
                let offset = object_transform.translation - ray.get_point(distance);

                commands.entity(placing_entity).insert((
                    object_transform,
                    CursorOffset(offset),
                    object_path.clone(),
                ));
            }
        }

        hover_settings.enabled = false;
    }

    pub(super) fn rotate(mut placing_objects: Query<(&mut Transform, &PlacingObject)>) {
        if let Ok((mut transform, placing_object)) = placing_objects.get_single_mut() {
            transform.rotate_y(placing_object.rotation_step);
        }
    }

    pub(super) fn apply_transform(
        spatial_query: SpatialQuery,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
        mut placing_objects: Query<(&mut Transform, &PlacingObject, &CursorOffset)>,
    ) {
        let Ok((mut transform, placing_object, cursor_offset)) = placing_objects.get_single_mut()
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

        let mut filter = SpatialQueryFilter::from_mask(Layer::Ground);
        if let PlacingObjectKind::Moving(entity) = placing_object.kind {
            filter.excluded_entities.insert(entity);
        }

        let Some(hit) = spatial_query.cast_ray(ray.origin, ray.direction, f32::MAX, false, filter)
        else {
            return;
        };

        let mut hit_position = ray.origin + ray.direction * hit.time_of_impact;
        hit_position.y = 0.0;
        transform.translation = hit_position + cursor_offset.0;
    }

    pub(super) fn check_collision(
        mut placing_objects: Query<(&mut PlacingObject, &CollidingEntities)>,
    ) {
        if let Ok((mut placing_object, colliding_entities)) = placing_objects.get_single_mut() {
            let mut collides = !colliding_entities.is_empty();
            if let PlacingObjectKind::Moving(entity) = placing_object.kind {
                if collides && colliding_entities.len() == 1 && colliding_entities.contains(&entity)
                {
                    // Ignore collision with the moving object.
                    collides = false;
                }
            }

            if placing_object.collides != collides {
                placing_object.collides = collides;
            }
        }
    }

    fn update_materials(
        mut materials: ResMut<Assets<StandardMaterial>>,
        placing_objects: Query<
            (Entity, &PlacingObject),
            Or<(Added<Children>, Changed<PlacingObject>)>,
        >,
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

                material.alpha_mode = AlphaMode::Add;
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

    fn confirm(
        mut move_events: EventWriter<ObjectMove>,
        mut buy_events: EventWriter<ObjectBuy>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(&Transform, &PlacingObject)>,
    ) {
        if let Ok((transform, placing_object)) = placing_objects.get_single() {
            if !placing_object.collides && placing_object.allowed_place {
                debug!("confirmed placing object {placing_object:?}");
                match placing_object.kind {
                    PlacingObjectKind::Spawning(id) => {
                        let metadata_path = asset_server
                            .get_path(id)
                            .expect("metadata should always come from file");
                        buy_events.send(ObjectBuy {
                            metadata_path: metadata_path.into_owned(),
                            position: transform.translation.xz(),
                            rotation: transform.rotation,
                        });
                    }
                    PlacingObjectKind::Moving(entity) => {
                        move_events.send(ObjectMove {
                            entity,
                            translation: transform.translation,
                            rotation: transform.rotation,
                        });
                    }
                }
            }
        }
    }

    fn sell(
        mut commands: Commands,
        mut sell_events: EventWriter<ObjectSell>,
        placing_objects: Query<(Entity, &PlacingObject)>,
    ) {
        if let Ok((entity, placing_object)) = placing_objects.get_single() {
            if let PlacingObjectKind::Moving(entity) = placing_object.kind {
                sell_events.send(ObjectSell(entity));
            }
            commands.entity(entity).despawn_recursive();
        }
    }

    fn cancel(
        mut commands: Commands,
        mut hover_settings: ResMut<CursorHoverSettings>,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        hover_settings.enabled = true;

        for placing_entity in &placing_objects {
            commands.entity(placing_entity).despawn_recursive();
        }
    }

    fn ensure_single(
        mut commands: Commands,
        new_placing_objects: Query<Entity, Added<PlacingObject>>,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        if let Some(new_entity) = new_placing_objects.iter().last() {
            for placing_entity in &placing_objects {
                if placing_entity != new_entity {
                    commands.entity(placing_entity).despawn_recursive();
                }
            }
        }
    }
}

#[derive(Component, Debug, Clone)]
pub(crate) struct PlacingObject {
    kind: PlacingObjectKind,
    collides: bool,
    pub(crate) allowed_place: bool,
    pub(crate) rotation_step: f32,
}

impl PlacingObject {
    pub(crate) fn moving(object_entity: Entity) -> Self {
        Self::new(PlacingObjectKind::Moving(object_entity))
    }

    pub(crate) fn spawning(id: AssetId<ObjectMetadata>) -> Self {
        Self::new(PlacingObjectKind::Spawning(id))
    }

    fn new(kind: PlacingObjectKind) -> Self {
        Self {
            kind,
            collides: false,
            allowed_place: true,
            rotation_step: FRAC_PI_4,
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
pub(super) struct CursorOffset(Vec3);
