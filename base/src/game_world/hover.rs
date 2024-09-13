pub(super) mod highlighting;

use std::iter;

use bevy::prelude::*;
use avian3d::prelude::*;

use super::{player_camera::CameraCaster, WorldState};
use highlighting::HighlightingPlugin;

pub(super) struct HoverPlugin;

impl Plugin for HoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HighlightingPlugin)
            .init_resource::<HoverEnabled>()
            .add_systems(
                PreUpdate,
                (
                    Self::raycast.pipe(Self::update).run_if(hover_enabled),
                    Self::cleanup
                        .run_if(resource_changed::<HoverEnabled>)
                        .run_if(not(hover_enabled)),
                )
                    .run_if(in_state(WorldState::City).or_else(in_state(WorldState::Family))),
            );
    }
}

impl HoverPlugin {
    fn raycast(
        spatial_query: SpatialQuery,
        camera_caster: CameraCaster,
        parents: Query<&Parent>,
        hoverable: Query<Entity, With<Hoverable>>,
    ) -> Option<(Entity, Vec3)> {
        let ray = camera_caster.ray()?;
        let hit = spatial_query.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            false,
            Default::default(),
        )?;

        let hovered_entity = hoverable
            .iter_many(iter::once(hit.entity).chain(parents.iter_ancestors(hit.entity)))
            .next()?;
        let point = ray.origin + ray.direction * hit.time_of_impact;

        Some((hovered_entity, point))
    }

    fn update(
        In(hit): In<Option<(Entity, Vec3)>>,
        mut commands: Commands,
        hovered: Query<Entity, With<Hovered>>,
    ) {
        match (hit, hovered.get_single().ok()) {
            (Some((hit_entity, point)), None) => {
                debug!("hovered `{hit_entity}`");
                commands.entity(hit_entity).insert(Hovered(point));
            }
            (None, Some(previous_entity)) => {
                debug!("unhovered `{previous_entity}`");
                commands.entity(previous_entity).remove::<Hovered>();
            }
            (Some((hit_entity, point)), Some(previous_entity)) => {
                commands.entity(hit_entity).insert(Hovered(point));
                if hit_entity != previous_entity {
                    debug!("changing hover from `{previous_entity}` to `{hit_entity}`");
                    commands.entity(previous_entity).remove::<Hovered>();
                }
            }
            (None, None) => (),
        }
    }

    fn cleanup(mut commands: Commands, hovered: Query<Entity, With<Hovered>>) {
        debug!("cleaning hover");
        if let Ok(hovered_entity) = hovered.get_single() {
            commands.entity(hovered_entity).remove::<Hovered>();
        }
    }

    pub(super) fn enable_on_remove<C: Component>(
        trigger: Trigger<OnRemove, C>,
        mut hover_enabled: ResMut<HoverEnabled>,
        other_compoents: Query<Entity, With<C>>,
    ) {
        if other_compoents
            .iter()
            .all(|entity| entity == trigger.entity())
        {
            hover_enabled.0 = true
        }
    }

    pub(super) fn disable_on_add<C: Component>(
        _trigger: Trigger<OnAdd, C>,
        mut hover_enabled: ResMut<HoverEnabled>,
    ) {
        hover_enabled.0 = false
    }
}

fn hover_enabled(hover_enabled: Res<HoverEnabled>) -> bool {
    hover_enabled.0
}

#[derive(Resource, Deref, DerefMut)]
pub(super) struct HoverEnabled(bool);

impl Default for HoverEnabled {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component)]
pub(super) struct Hoverable;

#[derive(Component, Deref)]
pub struct Hovered(pub(crate) Vec3);
