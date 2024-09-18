pub(crate) mod side_snap;
pub(crate) mod wall_snap;

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    fmt::Debug,
};

use avian3d::prelude::*;
use bevy::{
    color::palettes::css::{RED, WHITE},
    ecs::reflect::ReflectCommandExt,
    prelude::*,
    scene,
};
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::{
    asset::info::object_info::ObjectInfo,
    combined_scene_collider::CombinedSceneCollider,
    game_world::{
        city::CityMode,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::BuildingMode,
        hover::{HoverPlugin, Hovered},
        object::{Object, ObjectCommand},
        player_camera::{CameraCaster, PlayerCamera},
        Layer,
    },
    ghost::Ghost,
    settings::Action,
};
use side_snap::SideSnapPlugin;
use wall_snap::WallSnapPlugin;

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WallSnapPlugin)
            .add_plugins(SideSnapPlugin)
            .observe(HoverPlugin::enable_on_remove::<PlacingObject>)
            .observe(HoverPlugin::disable_on_add::<PlacingObject>)
            .observe(Self::ensure_single)
            .add_systems(
                PreUpdate,
                Self::init
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                Update,
                (
                    (
                        Self::pick
                            .run_if(action_just_pressed(Action::Confirm))
                            .run_if(not(any_with_component::<PlacingObject>)),
                        Self::sell.run_if(action_just_pressed(Action::Delete)),
                        Self::cancel.run_if(action_just_pressed(Action::Cancel)),
                    ),
                    (
                        Self::rotate.run_if(action_just_pressed(Action::RotateObject)),
                        Self::apply_position,
                        Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                    )
                        .chain(),
                )
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                SpawnScene,
                Self::update_materials
                    .after(scene::scene_spawner_system)
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            );
    }
}

impl PlacingObjectPlugin {
    fn pick(
        mut commands: Commands,
        objects: Query<(Entity, &Parent), (With<Object>, With<Hovered>)>,
    ) {
        if let Ok((object_entity, parent)) = objects.get_single() {
            info!("picking object `{object_entity}`");
            commands.entity(**parent).with_children(|parent| {
                parent.spawn(PlacingObject::moving(object_entity));
            });
        }
    }

    /// Inserts necessary components to trigger object initialization.
    fn init(
        mut commands: Commands,
        camera_caster: CameraCaster,
        objects_info: Res<Assets<ObjectInfo>>,
        asset_server: Res<AssetServer>,
        cameras: Query<&Transform, With<PlayerCamera>>,
        placing_objects: Query<(Entity, &PlacingObject), Without<PlacingObjectState>>,
        objects: Query<(&Object, &Transform)>,
    ) {
        let Some((placing_entity, &placing_object)) = placing_objects.iter().last() else {
            return;
        };

        debug!(
            "initializing placing object `{:?}` for `{placing_entity}`",
            placing_object.kind
        );

        let (info, cursor_offset, rotation) = match placing_object.kind {
            PlacingObjectKind::Spawning(id) => {
                let info = objects_info.get(id).expect("info should be preloaded");

                // Rotate towards camera and round to the nearest cardinal direction.
                let transform = cameras.single();
                let (y, ..) = transform.rotation.to_euler(EulerRot::YXZ);
                let rounded_angle = (y / FRAC_PI_2).round() * FRAC_PI_2 - PI;
                let rotation = Quat::from_rotation_y(rounded_angle);

                (info, Vec3::ZERO, rotation)
            }
            PlacingObjectKind::Moving(object_entity) => {
                let (object, &transform) = objects
                    .get(object_entity)
                    .expect("moving object should referece a valid object");

                let info_handle = asset_server
                    .get_handle(&object.0)
                    .expect("info should be preloaded");
                let info = objects_info.get(&info_handle).unwrap();

                let cursor_offset = camera_caster
                    .intersect_ground()
                    .map(|point| transform.translation - point)
                    .unwrap_or(transform.translation);

                (info, cursor_offset, transform.rotation)
            }
        };

        let scene_handle: Handle<Scene> = asset_server.load(info.scene.clone());
        let mut placing_entity = commands.entity(placing_entity);
        placing_entity.insert((
            Name::new("Placing object"),
            StateScoped(BuildingMode::Objects),
            StateScoped(CityMode::Objects),
            scene_handle,
            PlacingObjectState::new(cursor_offset),
            SpatialBundle::from_transform(Transform::from_rotation(rotation)),
            RigidBody::Kinematic,
            CombinedSceneCollider,
            CollisionLayers::new(
                Layer::PlacingObject,
                [Layer::Object, Layer::PlacingObject, Layer::Wall],
            ),
        ));

        if let PlacingObjectKind::Moving(object_entity) = placing_object.kind {
            placing_entity.insert(Ghost::new(object_entity).with_filters(Layer::PlacingObject));
        }

        for component in &info.components {
            placing_entity.insert_reflect(component.clone_value());
        }
        for component in &info.place_components {
            placing_entity.insert_reflect(component.clone_value());
        }
    }

    fn rotate(mut placing_objects: Query<(&mut Transform, &PlacingObject)>) {
        if let Ok((mut transform, object)) = placing_objects.get_single_mut() {
            transform.rotation *=
                Quat::from_axis_angle(Vec3::Y, object.rotation_limit.unwrap_or(FRAC_PI_4));

            debug!(
                "rotating placing object to '{}'",
                transform.rotation.to_euler(EulerRot::YXZ).0.to_degrees()
            );
        }
    }

    fn apply_position(
        camera_caster: CameraCaster,
        mut placing_objects: Query<(&mut Transform, &PlacingObjectState)>,
    ) {
        if let Ok((mut transform, state)) = placing_objects.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground() {
                transform.translation = point + state.cursor_offset;
            }
        }
    }

    fn update_materials(
        mut materials: ResMut<Assets<StandardMaterial>>,
        placing_objects: Query<
            (Entity, &PlacingObjectState, &CollidingEntities),
            Or<(Changed<CollidingEntities>, Changed<PlacingObjectState>)>,
        >,
        children: Query<&Children>,
        mut material_handles: Query<&mut Handle<StandardMaterial>>,
    ) {
        if let Ok((placing_entity, state, colliding_entities)) = placing_objects.get_single() {
            let color = if state.allowed_place && colliding_entities.is_empty() {
                WHITE.into()
            } else {
                RED.into()
            };
            debug!("changing base color to `{color:?}`");

            let mut iter =
                material_handles.iter_many_mut(children.iter_descendants(placing_entity));
            while let Some(mut material_handle) = iter.fetch_next() {
                let material = materials
                    .get(&*material_handle)
                    .expect("material handle should be valid");

                // If color matches, assume that we don't need any update.
                if material.base_color == color {
                    return;
                }

                let mut material = material.clone();
                material.base_color = color;
                material.alpha_mode = AlphaMode::Add;
                *material_handle = materials.add(material);
            }
        }
    }

    fn confirm(
        mut commands: Commands,
        mut history: CommandsHistory,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(
            Entity,
            &Parent,
            &Transform,
            &PlacingObject,
            &PlacingObjectState,
            &CollidingEntities,
        )>,
    ) {
        if let Ok((entity, parent, translation, &placing_object, state, colliding_entities)) =
            placing_objects.get_single()
        {
            if !state.allowed_place || !colliding_entities.is_empty() {
                return;
            }

            let id = match placing_object.kind {
                PlacingObjectKind::Spawning(id) => {
                    let info_path = asset_server
                        .get_path(id)
                        .expect("info should always come from file");
                    history.push_pending(ObjectCommand::Buy {
                        info_path: info_path.into_owned(),
                        city_entity: **parent,
                        translation: translation.translation,
                        rotation: translation.rotation,
                    })
                }
                PlacingObjectKind::Moving(entity) => history.push_pending(ObjectCommand::Move {
                    entity,
                    translation: translation.translation,
                    rotation: translation.rotation,
                }),
            };

            commands
                .entity(entity)
                .insert(PendingDespawn(id))
                .remove::<(PlacingObject, PlacingObjectState)>();

            info!("confirming `{:?}`", placing_object.kind);
        }
    }

    fn sell(
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_objects: Query<(Entity, &PlacingObject, &mut Transform)>,
        objects: Query<&Transform, Without<PlacingObject>>,
    ) {
        if let Ok((placing_entity, &placing_object, mut transform)) =
            placing_objects.get_single_mut()
        {
            info!("selling object");
            if let PlacingObjectKind::Moving(entity) = placing_object.kind {
                // Set original position until the deletion is confirmed.
                *transform = *objects.get(entity).expect("moving object should exist");

                let id = history.push_pending(ObjectCommand::Sell { entity });
                commands
                    .entity(placing_entity)
                    .insert(PendingDespawn(id))
                    .remove::<(PlacingObject, PlacingObjectState)>();
            } else {
                commands.entity(placing_entity).despawn_recursive();
            }
        }
    }

    fn cancel(mut commands: Commands, placing_objects: Query<Entity, With<PlacingObject>>) {
        if let Ok(placing_entity) = placing_objects.get_single() {
            info!("cancelling placing");
            commands.entity(placing_entity).despawn_recursive();
        }
    }

    fn ensure_single(
        trigger: Trigger<OnAdd, PlacingObject>,
        mut commands: Commands,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        for entity in placing_objects
            .iter()
            .filter(|&entity| entity != trigger.entity())
        {
            debug!("removing previous placing object `{entity}`");
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Clone, Copy, Component)]
pub struct PlacingObject {
    kind: PlacingObjectKind,
    rotation_limit: Option<f32>,
}

impl PlacingObject {
    pub fn spawning(id: AssetId<ObjectInfo>) -> Self {
        Self {
            kind: PlacingObjectKind::Spawning(id),
            rotation_limit: None,
        }
    }

    fn moving(entity: Entity) -> Self {
        Self {
            kind: PlacingObjectKind::Moving(entity),
            rotation_limit: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PlacingObjectKind {
    Spawning(AssetId<ObjectInfo>),
    Moving(Entity),
}

/// Controls if an object can be placed.
///
/// Stored as a separate component to avoid triggering change detection to update the object material.
#[derive(Component)]
struct PlacingObjectState {
    /// An offset between cursor position on first creation and object origin.
    cursor_offset: Vec3,

    /// Additional object condition for placing.
    ///
    /// For example, a door can be placed only on a wall. Controlled by other plugins.
    allowed_place: bool,
}

impl PlacingObjectState {
    fn new(cursor_offset: Vec3) -> Self {
        Self {
            cursor_offset,
            allowed_place: true,
        }
    }
}
