use avian3d::prelude::*;
use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

use super::{city::ActiveCity, player_camera::CameraCaster, WorldState};
use crate::{common_conditions::in_any_state, settings::Action};

pub(super) struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            Self::raycast
                .pipe(Self::trigger)
                .run_if(not(any_with_component::<Picked>))
                .run_if(in_any_state([WorldState::City, WorldState::Family])),
        );
    }
}

impl PickingPlugin {
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
        actions: Res<ActionState<Action>>,
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

        if let Some((entity, point)) = hit {
            if actions.just_pressed(&Action::Confirm) {
                commands.trigger_targets(Clicked(point), entity);
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

/// A component that disables the picking logic if present on any entity.
#[derive(Component)]
pub(super) struct Picked;
