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
};
use bevy_enhanced_input::prelude::*;

use crate::{
    alpha_color::{AlphaColor, AlphaColorPlugin},
    asset::info::object_info::ObjectInfo,
    common_conditions::observer_in_state,
    game_world::{
        city::CityMode,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::BuildingMode,
        object::{Object, ObjectCommand},
        picking::{Clicked, Picked},
        player_camera::{CameraCaster, PlayerCamera},
        Layer,
    },
    ghost::Ghost,
    settings::Settings,
};
use side_snap::SideSnapPlugin;
use wall_snap::WallSnapPlugin;

pub(super) struct PlacingObjectPlugin;

impl Plugin for PlacingObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(WallSnapPlugin)
            .add_plugins(SideSnapPlugin)
            .add_input_context::<PlacingObject>()
            .observe(Self::pick)
            .observe(Self::rotate)
            .observe(Self::sell)
            .observe(Self::cancel)
            .observe(Self::confirm)
            .observe(Self::ensure_single)
            .add_systems(
                PreUpdate,
                Self::init
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                Update,
                Self::apply_position
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                PostUpdate,
                Self::update_alpha
                    .before(AlphaColorPlugin::update_materials)
                    .after(PhysicsSet::StepSimulation)
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            );
    }
}

impl PlacingObjectPlugin {
    fn pick(
        trigger: Trigger<Clicked>,
        city_mode: Option<Res<State<CityMode>>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut commands: Commands,
        objects: Query<(Entity, &Parent), With<Object>>,
    ) {
        if !observer_in_state(city_mode, CityMode::Objects)
            && !observer_in_state(building_mode, BuildingMode::Objects)
        {
            return;
        }

        if let Ok((object_entity, parent)) = objects.get(trigger.entity()) {
            info!("picking object `{object_entity}`");
            commands.entity(**parent).with_children(|parent| {
                parent.spawn(PlacingObject::Moving(object_entity));
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

        debug!("initializing `{placing_object:?}` for `{placing_entity}`");

        let (info, cursor_offset, rotation) = match placing_object {
            PlacingObject::Spawning(id) => {
                let info = objects_info.get(id).expect("info should be preloaded");

                // Rotate towards camera and round to the nearest cardinal direction.
                let transform = cameras.single();
                let (y, ..) = transform.rotation.to_euler(EulerRot::YXZ);
                let rounded_angle = (y / FRAC_PI_2).round() * FRAC_PI_2 - PI;
                let rotation = Quat::from_rotation_y(rounded_angle);

                (info, Vec3::ZERO, rotation)
            }
            PlacingObject::Moving(object_entity) => {
                let (object, &transform) = objects
                    .get(object_entity)
                    .expect("moving object should reference a valid object");

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
            Picked,
            AlphaColor(WHITE.into()),
            scene_handle,
            PlacingObjectState::new(cursor_offset),
            ObjectRotationLimit::default(),
            SpatialBundle::from_transform(Transform::from_rotation(rotation)),
            RigidBody::Kinematic,
            CollisionLayers::new(
                Layer::PlacingObject,
                [
                    Layer::Object,
                    Layer::PlacingObject,
                    Layer::Wall,
                    Layer::PlacingWall,
                ],
            ),
        ));

        if let PlacingObject::Moving(object_entity) = placing_object {
            placing_entity.insert(Ghost::new(object_entity).with_filters(Layer::PlacingObject));
        }

        for component in &info.components {
            placing_entity.insert_reflect(component.clone_value());
        }
        for component in &info.place_components {
            placing_entity.insert_reflect(component.clone_value());
        }
    }

    fn rotate(
        trigger: Trigger<Started<RotateObject>>,
        city_mode: Option<Res<State<CityMode>>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut placing_objects: Query<(&mut Transform, &ObjectRotationLimit)>,
    ) {
        if !observer_in_state(city_mode, CityMode::Objects)
            && !observer_in_state(building_mode, BuildingMode::Objects)
        {
            return;
        }

        let Ok((mut transform, rotation_limit)) = placing_objects.get_single_mut() else {
            return;
        };

        let event = trigger.event();
        let angle = rotation_limit.unwrap_or(FRAC_PI_4) * event.value;
        transform.rotation *= Quat::from_axis_angle(Vec3::Y, angle);

        debug!(
            "rotating placing object to '{}'",
            transform.rotation.to_euler(EulerRot::YXZ).0.to_degrees()
        );
    }

    fn sell(
        _trigger: Trigger<Completed<SellObject>>,
        city_mode: Option<Res<State<CityMode>>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_objects: Query<(Entity, &PlacingObject, &mut Transform)>,
        objects: Query<&Transform, Without<PlacingObject>>,
    ) {
        if !observer_in_state(city_mode, CityMode::Objects)
            && !observer_in_state(building_mode, BuildingMode::Objects)
        {
            return;
        }

        let Ok((placing_entity, &placing_object, mut transform)) = placing_objects.get_single_mut()
        else {
            return;
        };

        info!("selling `{placing_object:?}`");
        if let PlacingObject::Moving(entity) = placing_object {
            // Set original position until the deletion is confirmed.
            *transform = *objects.get(entity).expect("moving object should exist");

            let command_id = history.push_pending(ObjectCommand::Sell { entity });
            commands
                .entity(placing_entity)
                .insert(PendingDespawn { command_id })
                .remove::<(PlacingObject, PlacingObjectState)>();
        } else {
            commands.entity(placing_entity).despawn_recursive();
        }
    }

    fn cancel(
        _trigger: Trigger<Completed<CancelObject>>,
        city_mode: Option<Res<State<CityMode>>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut commands: Commands,
        placing_objects: Query<Entity, With<PlacingObject>>,
    ) {
        if !observer_in_state(city_mode, CityMode::Objects)
            && !observer_in_state(building_mode, BuildingMode::Objects)
        {
            return;
        }

        if let Ok(placing_entity) = placing_objects.get_single() {
            info!("cancelling placing");
            commands.entity(placing_entity).despawn_recursive();
        }
    }

    fn confirm(
        _trigger: Trigger<Completed<ConfirmObject>>,
        city_mode: Option<Res<State<CityMode>>>,
        building_mode: Option<Res<State<BuildingMode>>>,
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
        if !observer_in_state(city_mode, CityMode::Objects)
            && !observer_in_state(building_mode, BuildingMode::Objects)
        {
            return;
        }

        let Ok((entity, parent, translation, &placing_object, state, colliding_entities)) =
            placing_objects.get_single()
        else {
            return;
        };

        if !state.allowed_place || !colliding_entities.is_empty() {
            return;
        }

        let command_id = match placing_object {
            PlacingObject::Spawning(id) => {
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
            PlacingObject::Moving(entity) => history.push_pending(ObjectCommand::Move {
                entity,
                translation: translation.translation,
                rotation: translation.rotation,
            }),
        };

        commands
            .entity(entity)
            .insert(PendingDespawn { command_id })
            .remove::<(PlacingObject, PlacingObjectState)>();

        info!("confirming `{placing_object:?}`");
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

    fn update_alpha(
        mut placing_objects: Query<
            (&mut AlphaColor, &PlacingObjectState, &CollidingEntities),
            Or<(Changed<CollidingEntities>, Changed<PlacingObjectState>)>,
        >,
    ) {
        let Ok((mut alpha, state, colliding_entities)) = placing_objects.get_single_mut() else {
            return;
        };

        if state.allowed_place && colliding_entities.is_empty() {
            **alpha = WHITE.into();
        } else {
            **alpha = RED.into();
        };
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
#[derive(Debug, Clone, Copy, Component)]
pub enum PlacingObject {
    Spawning(AssetId<ObjectInfo>),
    Moving(Entity),
}

impl InputContext for PlacingObject {
    const PRIORITY: isize = 1;

    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        ctx.bind::<RotateObject>().to((
            Biderectional {
                positive: &settings.keyboard.rotate_left,
                negative: &settings.keyboard.rotate_right,
            },
            MouseButton::Right,
            GamepadButtonType::West,
        ));
        ctx.bind::<SellObject>()
            .to((&settings.keyboard.delete, GamepadButtonType::North));
        ctx.bind::<CancelObject>()
            .to((KeyCode::Escape, GamepadButtonType::East));
        ctx.bind::<ConfirmObject>()
            .to((MouseButton::Left, GamepadButtonType::South));

        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = f32)]
struct RotateObject;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct SellObject;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct CancelObject;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct ConfirmObject;

#[derive(Component, Default, Deref, DerefMut)]
pub struct ObjectRotationLimit(Option<f32>);

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
