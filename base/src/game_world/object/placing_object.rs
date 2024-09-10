pub(crate) mod side_snap;
pub(crate) mod wall_snap;

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    fmt::Debug,
};

use bevy::{
    color::palettes::css::{RED, WHITE},
    ecs::{
        component::{ComponentHooks, StorageType},
        reflect::ReflectCommandExt,
    },
    prelude::*,
    scene,
};
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::{
    asset::info::object_info::ObjectInfo,
    combined_scene_collider::CombinedSceneCollider,
    game_world::{
        city::CityMode,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::BuildingMode,
        hover::{HoverEnabled, Hovered},
        object::{Object, ObjectCommand},
        player_camera::CameraCaster,
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
            )
            .add_systems(
                PostUpdate,
                Self::ensure_single
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
        placing_objects: Query<(Entity, &PlacingObject), Without<PlacingObjectState>>,
        objects: Query<(&Object, &Position, &Rotation)>,
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
                let (transform, _) = camera_caster.cameras.single();
                let (_, rotation, _) = transform.to_scale_rotation_translation();
                let (y, ..) = rotation.to_euler(EulerRot::YXZ);
                let rounded_angle = (y / FRAC_PI_2).round() * FRAC_PI_2 - PI;
                let rotation = Rotation(Quat::from_rotation_y(rounded_angle));

                (info, Vec3::ZERO, rotation)
            }
            PlacingObjectKind::Moving(object_entity) => {
                let (object, &position, &rotation) = objects
                    .get(object_entity)
                    .expect("moving object should referece a valid object");

                let info_handle = asset_server
                    .get_handle(&object.0)
                    .expect("info should be preloaded");
                let info = objects_info.get(&info_handle).unwrap();

                let cursor_offset = camera_caster
                    .intersect_ground()
                    .map(|point| *position - point)
                    .unwrap_or(*position);

                (info, cursor_offset, rotation)
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
            rotation,
            Position::default(),
            RigidBody::Kinematic,
            CombinedSceneCollider,
            SpatialBundle::default(),
            CollisionLayers::new(
                Layer::PlacingObject,
                [Layer::Object, Layer::PlacingObject, Layer::Wall],
            ),
        ));

        if let PlacingObjectKind::Moving(object_entity) = placing_object.kind {
            placing_entity.insert(Ghost::new(object_entity, Layer::PlacingObject));
        }

        for component in &info.components {
            placing_entity.insert_reflect(component.clone_value());
        }
        for component in &info.place_components {
            placing_entity.insert_reflect(component.clone_value());
        }
    }

    fn rotate(mut placing_objects: Query<(&mut Rotation, &PlacingObject)>) {
        if let Ok((mut rotation, object)) = placing_objects.get_single_mut() {
            **rotation *=
                Quat::from_axis_angle(Vec3::Y, object.rotation_limit.unwrap_or(FRAC_PI_4));

            debug!(
                "rotating placing object to '{}'",
                rotation.to_euler(EulerRot::YXZ).0.to_degrees()
            );
        }
    }

    fn apply_position(
        camera_caster: CameraCaster,
        mut placing_objects: Query<(&mut Position, &PlacingObjectState)>,
    ) {
        if let Ok((mut position, state)) = placing_objects.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground() {
                **position = point + state.cursor_offset;
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
            &Position,
            &Rotation,
            &PlacingObject,
            &PlacingObjectState,
            &CollidingEntities,
        )>,
    ) {
        if let Ok((entity, position, rotation, &placing_object, state, colliding_entities)) =
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
                        position: **position,
                        rotation: **rotation,
                    })
                }
                PlacingObjectKind::Moving(entity) => history.push_pending(ObjectCommand::Move {
                    entity,
                    position: **position,
                    rotation: **rotation,
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
        mut placing_objects: Query<(Entity, &PlacingObject, &mut Position, &mut Rotation)>,
        objects: Query<(&Position, &Rotation), Without<PlacingObject>>,
    ) {
        if let Ok((placing_entity, &placing_object, mut position, mut rotation)) =
            placing_objects.get_single_mut()
        {
            info!("selling object");
            if let PlacingObjectKind::Moving(entity) = placing_object.kind {
                let id = history.push_pending(ObjectCommand::Sell { entity });

                let (original_position, original_rotation) =
                    objects.get(entity).expect("moving object should exist");
                *position = *original_position;
                *rotation = *original_rotation;

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
        mut commands: Commands,
        new_placing_objects: Query<Entity, Added<PlacingObject>>,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        if let Some(new_entity) = new_placing_objects.iter().last() {
            for placing_entity in &placing_objects {
                if placing_entity != new_entity {
                    debug!("removing previous placing object `{placing_entity}`");
                    commands.entity(placing_entity).despawn_recursive();
                }
            }
        }
    }
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Clone, Copy)]
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

impl Component for PlacingObject {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks
            .on_add(|mut world, _targeted_entity, _component_id| {
                **world.resource_mut::<HoverEnabled>() = false;
            })
            .on_remove(|mut world, _targeted_entity, _component_id| {
                **world.resource_mut::<HoverEnabled>() = true;
            });
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
