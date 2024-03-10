use std::{
    f32::consts::{FRAC_PI_4, PI},
    fmt::Debug,
};

use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_replicon::prelude::*;
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
    player_camera::CameraCaster,
};

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(CityMode::Objects), Self::end_placing)
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
                                in_state(GameState::Family)
                                    .and_then(in_state(FamilyMode::Building)),
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
                        Self::apply_transform,
                        Self::check_collision,
                        (
                            Self::update_materials,
                            Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                        ),
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
                (Self::init, Self::ensure_single).run_if(
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
        camera_caster: CameraCaster,
        mut hover_settings: ResMut<CursorHoverSettings>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(Entity, &PlacingObject), Added<PlacingObject>>,
        objects: Query<(&Transform, &ObjectPath)>,
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

                let offset = camera_caster
                    .intersect_ground()
                    .map(|point| object_transform.translation - point)
                    .unwrap_or(object_transform.translation);

                commands.entity(placing_entity).insert((
                    object_transform,
                    CursorOffset(offset),
                    object_path.clone(),
                ));
            }
        }

        hover_settings.enabled = false;
    }

    pub(super) fn rotate(
        mut placing_objects: Query<(&mut Transform, &PlacingObject), Without<UnconfirmedObject>>,
    ) {
        if let Ok((mut transform, placing_object)) = placing_objects.get_single_mut() {
            transform.rotate_y(placing_object.rotation_step);
        }
    }

    pub(super) fn apply_transform(
        camera_caster: CameraCaster,
        mut placing_objects: Query<(&mut Transform, &CursorOffset), Without<UnconfirmedObject>>,
    ) {
        if let Ok((mut transform, cursor_offset)) = placing_objects.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground() {
                transform.translation = point + cursor_offset.0;
            }
        }
    }

    pub(super) fn check_collision(
        mut placing_objects: Query<
            (&mut PlacingObject, &CollidingEntities),
            Without<UnconfirmedObject>,
        >,
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
            (
                Or<(Added<Children>, Changed<PlacingObject>)>,
                Without<UnconfirmedObject>,
            ),
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
        mut commands: Commands,
        mut move_events: EventWriter<ObjectMove>,
        mut buy_events: EventWriter<ObjectBuy>,
        asset_server: Res<AssetServer>,
        placing_objects: Query<(Entity, &Transform, &PlacingObject), Without<UnconfirmedObject>>,
    ) {
        if let Ok((entity, transform, placing_object)) = placing_objects.get_single() {
            if !placing_object.collides && placing_object.allowed_place {
                commands.entity(entity).insert(UnconfirmedObject);

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

                debug!("requested confirmation for {placing_object:?}");
            }
        }
    }

    fn sell(
        mut commands: Commands,
        mut sell_events: EventWriter<ObjectSell>,
        placing_objects: Query<(Entity, &PlacingObject), Without<UnconfirmedObject>>,
    ) {
        if let Ok((entity, placing_object)) = placing_objects.get_single() {
            if let PlacingObjectKind::Moving(entity) = placing_object.kind {
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

#[derive(Component)]
pub(crate) struct UnconfirmedObject;
