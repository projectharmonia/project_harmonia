pub(crate) mod side_snap;
pub(crate) mod wall_snap;

use std::{
    f32::consts::{FRAC_PI_2, FRAC_PI_4, PI},
    fmt::Debug,
};

use bevy::{asset::AssetPath, prelude::*, scene};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use crate::{
    asset::metadata::object_metadata::ObjectMetadata,
    city::CityMode,
    cursor_hover::{CursorHover, CursorHoverSettings},
    family::{BuildingMode, FamilyMode},
    core::GameState,
    object::{ObjectBuy, ObjectEventConfirmed, ObjectMove, ObjectPath, ObjectSell},
    player_camera::CameraCaster,
    settings::Action,
};
use side_snap::SideSnapPlugin;
use wall_snap::WallSnapPlugin;

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WallSnapPlugin)
            .add_plugins(SideSnapPlugin)
            .add_systems(OnExit(CityMode::Objects), Self::end_placing)
            .add_systems(OnExit(FamilyMode::Building), Self::end_placing)
            .add_systems(
                PreUpdate,
                Self::end_placing
                    .after(ClientSet::Receive)
                    .run_if(on_event::<ObjectEventConfirmed>())
                    .run_if(
                        in_state(GameState::City)
                            .and_then(in_state(CityMode::Objects))
                            .or_else(
                                in_state(GameState::Family).and_then(
                                    in_state(FamilyMode::Building)
                                        .and_then(in_state(BuildingMode::Objects)),
                                ),
                            ),
                    ),
            )
            .add_systems(
                Update,
                (
                    (
                        Self::pick
                            .run_if(action_just_pressed(Action::Confirm))
                            .run_if(not(any_with_component::<PlacingObject>)),
                        Self::sell.run_if(action_just_pressed(Action::Delete)),
                        Self::end_placing.run_if(action_just_pressed(Action::Cancel)),
                    ),
                    (
                        Self::rotate.run_if(action_just_pressed(Action::RotateObject)),
                        Self::apply_position,
                        Self::check_collision,
                        Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                    )
                        .chain(),
                )
                    .run_if(
                        in_state(GameState::City)
                            .and_then(in_state(CityMode::Objects))
                            .or_else(
                                in_state(GameState::Family).and_then(
                                    in_state(FamilyMode::Building)
                                        .and_then(in_state(BuildingMode::Objects)),
                                ),
                            ),
                    ),
            )
            .add_systems(
                SpawnScene,
                Self::update_materials
                    .after(scene::scene_spawner_system)
                    .run_if(
                        in_state(GameState::City)
                            .and_then(in_state(CityMode::Objects))
                            .or_else(
                                in_state(GameState::Family).and_then(
                                    in_state(FamilyMode::Building)
                                        .and_then(in_state(BuildingMode::Objects)),
                                ),
                            ),
                    ),
            )
            .add_systems(
                PostUpdate,
                (Self::init, Self::ensure_single).run_if(
                    in_state(GameState::City)
                        .and_then(in_state(CityMode::Objects))
                        .or_else(
                            in_state(GameState::Family).and_then(
                                in_state(FamilyMode::Building)
                                    .and_then(in_state(BuildingMode::Objects)),
                            ),
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
                parent.spawn(PlacingObject::Moving(placing_entity));
            });
        }
    }

    /// Inserts necessary components to trigger object initialization.
    fn init(
        mut commands: Commands,
        camera_caster: CameraCaster,
        mut hover_settings: ResMut<CursorHoverSettings>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(Entity, &PlacingObject), Added<PlacingObject>>,
        objects: Query<(&Position, &Rotation, &ObjectPath)>,
    ) {
        let Some((placing_entity, &placing_object)) = placing_objects.iter().last() else {
            return;
        };

        debug!("creating {placing_object:?}");
        match placing_object {
            PlacingObject::Spawning(id) => {
                let metadata_path = asset_server
                    .get_path(id)
                    .expect("metadata should always come from file");

                // Rotate towards camera and round to the nearest cardinal direction.
                let (transform, _) = camera_caster.cameras.single();
                let (_, rotation, _) = transform.to_scale_rotation_translation();
                let (y, ..) = rotation.to_euler(EulerRot::YXZ);
                let rounded_angle = (y / FRAC_PI_2).round() * FRAC_PI_2 - PI;

                commands
                    .entity(placing_entity)
                    .insert(PlacingInitBundle::spawning(
                        metadata_path.into_owned(),
                        rounded_angle,
                    ));
            }
            PlacingObject::Moving(object_entity) => {
                let (&position, &rotation, object_path) = objects
                    .get(object_entity)
                    .expect("moving object should have scene and path");

                let offset = camera_caster
                    .intersect_ground()
                    .map(|point| *position - point)
                    .unwrap_or(*position);

                commands
                    .entity(placing_entity)
                    .insert(PlacingInitBundle::moving(
                        object_path.clone(),
                        CursorOffset(offset),
                        position,
                        rotation,
                    ));
            }
        }

        hover_settings.enabled = false;
    }

    fn rotate(
        mut placing_objects: Query<(&mut Rotation, &RotationLimit), Without<UnconfirmedObject>>,
    ) {
        if let Ok((mut rotation, limit)) = placing_objects.get_single_mut() {
            **rotation *= Quat::from_axis_angle(Vec3::Y, limit.unwrap_or(FRAC_PI_4));
        }
    }

    fn apply_position(
        camera_caster: CameraCaster,
        mut placing_objects: Query<(&mut Position, &CursorOffset), Without<UnconfirmedObject>>,
    ) {
        if let Ok((mut position, cursor_offset)) = placing_objects.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground() {
                **position = point + cursor_offset.0;
            }
        }
    }

    fn check_collision(
        mut placing_objects: Query<
            (&mut PlaceState, &PlacingObject, &CollidingEntities),
            Without<UnconfirmedObject>,
        >,
    ) {
        if let Ok((mut state, &placing_object, colliding_entities)) =
            placing_objects.get_single_mut()
        {
            let mut collides = !colliding_entities.is_empty();
            if let PlacingObject::Moving(entity) = placing_object {
                if colliding_entities.len() == 1 && colliding_entities.contains(&entity) {
                    // Ignore collision with the moving object.
                    collides = false;
                }
            }

            if state.collides != collides {
                state.collides = collides;
            }
        }
    }

    fn update_materials(
        mut materials: ResMut<Assets<StandardMaterial>>,
        placing_objects: Query<
            (Entity, &PlaceState),
            (
                Or<(Added<Children>, Changed<PlaceState>)>,
                Without<UnconfirmedObject>,
            ),
        >,
        children: Query<&Children>,
        mut material_handles: Query<&mut Handle<StandardMaterial>>,
    ) {
        if let Ok((placing_entity, state)) = placing_objects.get_single() {
            let color = if state.placeable() {
                Color::WHITE
            } else {
                Color::RED
            };

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
        mut move_events: EventWriter<ObjectMove>,
        mut buy_events: EventWriter<ObjectBuy>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<
            (Entity, &Position, &Rotation, &PlacingObject, &PlaceState),
            Without<UnconfirmedObject>,
        >,
    ) {
        if let Ok((entity, position, rotation, &placing_object, state)) =
            placing_objects.get_single()
        {
            if state.placeable() {
                commands.entity(entity).insert(UnconfirmedObject);

                match placing_object {
                    PlacingObject::Spawning(id) => {
                        let metadata_path = asset_server
                            .get_path(id)
                            .expect("metadata should always come from file");
                        buy_events.send(ObjectBuy {
                            metadata_path: metadata_path.into_owned(),
                            position: **position,
                            rotation: **rotation,
                        });
                    }
                    PlacingObject::Moving(entity) => {
                        move_events.send(ObjectMove {
                            entity,
                            position: **position,
                            rotation: **rotation,
                        });
                    }
                }

                debug!("requested confirmation for {placing_object:?}");
            }
        }
    }

    fn sell(
        mut commands: Commands,
        mut sell_events: EventWriter<ObjectSell>,
        placing_objects: Query<(Entity, &PlacingObject), Without<UnconfirmedObject>>,
    ) {
        if let Ok((entity, &placing_object)) = placing_objects.get_single() {
            if let PlacingObject::Moving(entity) = placing_object {
                sell_events.send(ObjectSell(entity));
            }
            commands.entity(entity).despawn_recursive();
        }
    }

    fn end_placing(
        mut commands: Commands,
        mut hover_settings: ResMut<CursorHoverSettings>,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        if let Ok(placing_entity) = placing_objects.get_single() {
            hover_settings.enabled = true;
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

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Component, Debug, Clone, Copy)]
pub enum PlacingObject {
    Spawning(AssetId<ObjectMetadata>),
    Moving(Entity),
}

/// Additional components that needed for [`PlacingObject`].
#[derive(Bundle)]
struct PlacingInitBundle {
    object_path: ObjectPath,
    cursor_offset: CursorOffset,
    position: Position,
    rotation: Rotation,
    state: PlaceState,
    snapped: RotationLimit,
}

impl PlacingInitBundle {
    fn spawning(metadata_path: AssetPath<'static>, angle: f32) -> Self {
        Self {
            object_path: ObjectPath(metadata_path.into_owned()),
            cursor_offset: Default::default(),
            position: Default::default(),
            rotation: Rotation(Quat::from_rotation_y(angle)),
            state: Default::default(),
            snapped: Default::default(),
        }
    }

    fn moving(
        object_path: ObjectPath,
        cursor_offset: CursorOffset,
        position: Position,
        rotation: Rotation,
    ) -> Self {
        Self {
            object_path,
            cursor_offset,
            position,
            rotation,
            state: PlaceState::default(),
            snapped: Default::default(),
        }
    }
}

/// Contains an offset between cursor position on first creation and object origin.
#[derive(Clone, Component, Copy, Default, Deref)]
struct CursorOffset(Vec3);

/// Controls if an object can be placed.
///
/// Stored as a separate component to avoid triggering change detection to update the object material.
#[derive(Component)]
struct PlaceState {
    /// Can be placed without colliding with any other entities.
    collides: bool,

    /// Additional object condition for placing.
    ///
    /// For example, a door can be placed only on a wall. Controlled by other plugins.
    allowed_place: bool,
}

impl PlaceState {
    fn placeable(&self) -> bool {
        !self.collides && self.allowed_place
    }
}

impl Default for PlaceState {
    fn default() -> Self {
        Self {
            allowed_place: true,
            collides: false,
        }
    }
}

/// Limits object rotation to the specified angle if set.
#[derive(Component, Default, Deref)]
struct RotationLimit(Option<f32>);

#[derive(Component)]
struct UnconfirmedObject;
