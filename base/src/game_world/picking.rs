use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use super::{city::ActiveCity, player_camera::CameraCaster, WorldState};
use crate::common_conditions::in_any_state;

pub(super) struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<PickerInput>()
            .add_systems(OnEnter(WorldState::City), Self::spawn)
            .add_systems(OnEnter(WorldState::Family), Self::spawn)
            .add_systems(
                PreUpdate,
                Self::raycast
                    .pipe(Self::trigger)
                    .run_if(not(any_with_component::<Picked>))
                    .run_if(in_any_state([WorldState::City, WorldState::Family])),
            );
    }
}

impl PickingPlugin {
    fn spawn(mut commands: Commands, world_state: Res<State<WorldState>>) {
        commands.spawn((PickerInput, StateScoped(**world_state)));
    }

    fn raycast(
        spatial_query: SpatialQuery,
        camera_caster: CameraCaster,
        active_cities: Query<&GlobalTransform, With<ActiveCity>>,
    ) -> Option<(Entity, Vec3)> {
        let ray = camera_caster.ray()?;
        let hit = spatial_query.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            false,
            Default::default(),
        )?;

        let city_transform = active_cities.single();
        let global_point = ray.origin + ray.direction * hit.time_of_impact;
        let point = city_transform
            .affine()
            .inverse()
            .transform_point3(global_point);

        Some((hit.entity, point))
    }

    fn trigger(
        In(hit): In<Option<(Entity, Vec3)>>,
        mut last_hover: Local<Option<Entity>>,
        instances: Res<ContextInstances>,
        pickers: Query<Entity, With<PickerInput>>,
        mut commands: Commands,
    ) {
        let current_hover = hit.map(|(entity, _)| entity);
        match (current_hover, *last_hover) {
            (Some(current_entity), None) => {
                debug!("hovered `{current_entity}`");
                commands.trigger_targets(Hovered, current_entity);
            }
            (None, Some(last_entity)) => {
                debug!("unhovered `{last_entity}`");
                commands.trigger_targets(Unhovered, last_entity);
            }
            (Some(current_entity), Some(last_entity)) => {
                if current_entity != last_entity {
                    debug!("changing hover from `{last_entity}` to `{current_entity}`");
                    commands.trigger_targets(Unhovered, last_entity);
                    commands.trigger_targets(Hovered, current_entity);
                }
            }
            (None, None) => (),
        }

        *last_hover = current_hover;

        if let Some((hit_entity, point)) = hit {
            let picker_entity = pickers.single();
            let ctx = instances.get::<PickerInput>(picker_entity).unwrap();
            let action = ctx.action::<Pick>().unwrap();
            if action.events().contains(ActionEvents::COMPLETED) {
                commands.trigger_targets(Clicked(point), hit_entity);
            }
        }
    }
}

/// Triggered when a pickable entity gets hovered.
#[derive(Event)]
pub(super) struct Hovered;

/// Triggered when a pickable entity gets unhovered.
#[derive(Event)]
pub(super) struct Unhovered;

/// Triggered when clicked on a pickable entity.
#[derive(Event, Deref)]
pub struct Clicked(pub Vec3);

/// A component that disables the raycasting logic if present on any entity.
#[derive(Component)]
pub(super) struct Picked;

/// Reads input for picking.
#[derive(Component)]
struct PickerInput;

impl InputContext for PickerInput {
    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();

        ctx.bind::<Pick>()
            .to((MouseButton::Left, GamepadButtonType::South));

        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct Pick;
