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
    alpha_color::{self, AlphaColor},
    asset::manifest::object_manifest::ObjectManifest,
    game_world::{
        city::CityMode,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::BuildingMode,
        highlighting::HighlightDisabler,
        object::{Object, ObjectCommand},
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
            .add_observer(pick)
            .add_observer(init)
            .add_observer(rotate)
            .add_observer(sell)
            .add_observer(cancel.never_param_warn())
            .add_observer(confirm)
            .add_systems(
                Update,
                apply_position
                    .never_param_warn()
                    .run_if(in_state(CityMode::Objects).or(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                PostUpdate,
                update_alpha
                    .never_param_warn()
                    .before(alpha_color::update_materials)
                    .run_if(in_state(CityMode::Objects).or(in_state(BuildingMode::Objects))),
            );
    }
}

fn pick(
    mut trigger: Trigger<Pointer<Click>>,
    city_mode: Option<Res<State<CityMode>>>,
    building_mode: Option<Res<State<BuildingMode>>>,
    mut commands: Commands,
    objects: Query<(Entity, &Parent), With<Object>>,
    placing_objects: Query<(), With<PlacingObject>>,
) {
    if trigger.button != PointerButton::Primary {
        return;
    }
    if city_mode.is_some_and(|mode| **mode != CityMode::Objects)
        && building_mode.is_some_and(|mode| **mode != BuildingMode::Objects)
    {
        return;
    }
    if !placing_objects.is_empty() {
        return;
    }
    let Ok((object_entity, parent)) = objects.get(trigger.entity()) else {
        return;
    };
    trigger.propagate(false);

    info!("picking object `{object_entity}`");
    commands.entity(**parent).with_children(|parent| {
        parent.spawn(PlacingObject::Moving(object_entity));
    });
}

/// Inserts necessary components to trigger object initialization.
fn init(
    trigger: Trigger<OnAdd, PlacingObject>,
    mut commands: Commands,
    camera_caster: CameraCaster,
    objects_manifests: Res<Assets<ObjectManifest>>,
    asset_server: Res<AssetServer>,
    camera_transform: Single<&Transform, With<PlayerCamera>>,
    mut placing_objects: Query<
        (
            &PlacingObject,
            &mut SceneRoot,
            &mut PlacingObjectState,
            &mut Transform,
        ),
        Without<PlayerCamera>,
    >,
    objects: Query<(&Object, &Transform), Without<PlacingObject>>,
) {
    let (&placing_object, mut scene_root, mut state, mut transform) =
        placing_objects.get_mut(trigger.entity()).unwrap();

    debug!(
        "initializing `{placing_object:?}` for `{}`",
        trigger.entity()
    );

    let (manifest, cursor_offset, rotation) = match placing_object {
        PlacingObject::Spawning(id) => {
            let manifest = objects_manifests.get(id).expect("info should be preloaded");

            // Rotate towards camera and round to the nearest cardinal direction.
            let (y, ..) = camera_transform.rotation.to_euler(EulerRot::YXZ);
            let rounded_angle = (y / FRAC_PI_2).round() * FRAC_PI_2 - PI;
            let rotation = Quat::from_rotation_y(rounded_angle);

            (manifest, Vec3::ZERO, rotation)
        }
        PlacingObject::Moving(object_entity) => {
            let (object, &transform) = objects
                .get(object_entity)
                .expect("moving object should reference a valid object");

            let manifest_handle = asset_server
                .get_handle(&**object)
                .expect("info should be preloaded");
            let manifest = objects_manifests.get(&manifest_handle).unwrap();

            let cursor_offset = camera_caster
                .intersect_ground()
                .map(|point| transform.translation - point)
                .unwrap_or(transform.translation);

            (manifest, cursor_offset, transform.rotation)
        }
    };

    scene_root.0 = asset_server.load(manifest.scene.clone());
    transform.rotation = rotation;
    state.cursor_offset = cursor_offset;

    let mut placing_entity = commands.entity(trigger.entity());

    if let PlacingObject::Moving(object_entity) = placing_object {
        placing_entity.insert(Ghost::new(object_entity).with_filters(Layer::PlacingObject));
    }

    for component in &manifest.components {
        placing_entity.insert_reflect(component.clone_value());
    }
    for component in &manifest.place_components {
        placing_entity.insert_reflect(component.clone_value());
    }
}

fn rotate(
    trigger: Trigger<Started<RotateObject>>,
    placing_object: Single<(&mut Transform, &ObjectRotationLimit)>,
) {
    let (mut transform, rotation_limit) = placing_object.into_inner();
    let angle = rotation_limit.unwrap_or(FRAC_PI_4) * trigger.value;
    transform.rotation *= Quat::from_axis_angle(Vec3::Y, angle);

    debug!(
        "rotating placing object to '{}'",
        transform.rotation.to_euler(EulerRot::YXZ).0.to_degrees()
    );
}

fn sell(
    trigger: Trigger<Completed<SellObject>>,
    mut commands: Commands,
    mut history: CommandsHistory,
    placing_object: Single<&PlacingObject>,
) {
    info!("selling `{:?}`", trigger.entity());
    if let PlacingObject::Moving(entity) = **placing_object {
        let command_id = history.push_pending(ObjectCommand::Sell { entity });
        commands
            .entity(trigger.entity())
            .insert(PendingDespawn { command_id })
            .remove::<(PlacingObject, PlacingObjectState)>();
    } else {
        commands.entity(trigger.entity()).despawn_recursive();
    }
}

fn cancel(trigger: Trigger<Completed<CancelObject>>, mut commands: Commands) {
    info!("cancelling placing");
    commands.entity(trigger.entity()).despawn_recursive();
}

fn confirm(
    trigger: Trigger<Completed<ConfirmObject>>,
    mut commands: Commands,
    mut history: CommandsHistory,
    asset_server: Res<AssetServer>,
    placing_object: Single<(
        &Parent,
        &Transform,
        &PlacingObject,
        &PlacingObjectState,
        &CollidingEntities,
    )>,
) {
    let (parent, translation, &placing_object, state, colliding_entities) = *placing_object;

    if !state.allowed_place || !colliding_entities.is_empty() {
        return;
    }

    let command_id = match placing_object {
        PlacingObject::Spawning(id) => {
            let manifest_path = asset_server
                .get_path(id)
                .expect("manifest should always come from file");
            history.push_pending(ObjectCommand::Buy {
                manifest_path: manifest_path.into_owned(),
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
        .entity(trigger.entity())
        .insert(PendingDespawn { command_id })
        .remove::<(PlacingObject, PlacingObjectState)>();

    info!("confirming `{placing_object:?}`");
}

fn apply_position(
    camera_caster: CameraCaster,
    placing_object: Single<(&mut Transform, &PlacingObjectState)>,
) {
    let (mut transform, state) = placing_object.into_inner();
    if let Some(point) = camera_caster.intersect_ground() {
        transform.translation = point + state.cursor_offset;
    }
}

fn update_alpha(
    placing_object: Single<
        (&mut AlphaColor, &PlacingObjectState, &CollidingEntities),
        Or<(Changed<CollidingEntities>, Changed<PlacingObjectState>)>,
    >,
) {
    let (mut alpha, state, colliding_entities) = placing_object.into_inner();
    if state.allowed_place && colliding_entities.is_empty() {
        **alpha = WHITE.into();
    } else {
        **alpha = RED.into();
    };
}

/// Marks an entity as an object that should be moved with cursor to preview spawn position.
#[derive(Debug, Clone, Copy, Component)]
#[require(
    Name(|| Name::new("Placing object")),
    PlacingObjectState,
    ObjectRotationLimit,
    StateScoped::<BuildingMode>(|| StateScoped(BuildingMode::Objects)),
    StateScoped::<CityMode>(|| StateScoped(CityMode::Objects)),
    HighlightDisabler,
    AlphaColor(|| AlphaColor(WHITE.into())),
    SceneRoot,
    RigidBody(|| RigidBody::Kinematic),
    CollidingEntities,
    CollisionLayers(|| CollisionLayers::new(
        Layer::PlacingObject,
        [
            Layer::Object,
            Layer::PlacingObject,
            Layer::Wall,
            Layer::PlacingWall,
        ],
    )),
)]
pub enum PlacingObject {
    Spawning(AssetId<ObjectManifest>),
    Moving(Entity),
}

impl InputContext for PlacingObject {
    const PRIORITY: isize = 1;

    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        ctx.bind::<RotateObject>().to((
            Bidirectional {
                positive: &settings.keyboard.rotate_left,
                negative: &settings.keyboard.rotate_right,
            },
            MouseButton::Right,
            GamepadButton::West,
        ));
        ctx.bind::<SellObject>()
            .to((&settings.keyboard.delete, GamepadButton::North));
        ctx.bind::<CancelObject>()
            .to((KeyCode::Escape, GamepadButton::East));
        ctx.bind::<ConfirmObject>()
            .to((MouseButton::Left, GamepadButton::South));

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

impl Default for PlacingObjectState {
    fn default() -> Self {
        Self {
            cursor_offset: Default::default(),
            allowed_place: true,
        }
    }
}
