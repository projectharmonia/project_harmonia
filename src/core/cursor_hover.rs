use std::iter;

use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use super::{game_state::GameState, player_camera::CameraCaster};

pub(super) struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorHoverSettings>().add_systems(
            PreUpdate,
            (
                Self::raycast
                    .pipe(Self::update)
                    .run_if(cursor_hover_enabled),
                Self::remove_all
                    .run_if(resource_changed::<CursorHoverSettings>)
                    .run_if(not(cursor_hover_enabled)),
            )
                .run_if(in_state(GameState::City).or_else(in_state(GameState::Family))),
        );
    }
}

impl CursorHoverPlugin {
    fn raycast(
        spatial_query: SpatialQuery,
        camera_caster: CameraCaster,
        parents: Query<&Parent>,
        cursor_hoverable: Query<Entity, With<CursorHoverable>>,
    ) -> Option<(Entity, Vec3)> {
        let ray = camera_caster.ray()?;
        let hit = spatial_query.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            false,
            Default::default(),
        )?;

        let hovered_entity = cursor_hoverable
            .iter_many(iter::once(hit.entity).chain(parents.iter_ancestors(hit.entity)))
            .next()?;
        let point = ray.origin + ray.direction * hit.time_of_impact;

        Some((hovered_entity, point))
    }

    fn update(
        In(hit): In<Option<(Entity, Vec3)>>,
        mut commands: Commands,
        cursor_hovers: Query<Entity, With<CursorHover>>,
    ) {
        match (hit, cursor_hovers.get_single().ok()) {
            (Some((hit_entity, point)), None) => {
                commands.entity(hit_entity).insert(CursorHover(point));
            }
            (None, Some(previous_entity)) => {
                commands.entity(previous_entity).remove::<CursorHover>();
            }
            (Some((hit_entity, point)), Some(previous_entity)) => {
                commands.entity(hit_entity).insert(CursorHover(point));
                if hit_entity != previous_entity {
                    commands.entity(previous_entity).remove::<CursorHover>();
                }
            }
            (None, None) => (),
        }
    }

    fn remove_all(mut commands: Commands, hovered: Query<Entity, With<CursorHover>>) {
        if let Ok(hovered_entity) = hovered.get_single() {
            commands.entity(hovered_entity).remove::<CursorHover>();
        }
    }
}

fn cursor_hover_enabled(hover_settings: Res<CursorHoverSettings>) -> bool {
    hover_settings.enabled
}

#[derive(Resource)]
pub(super) struct CursorHoverSettings {
    pub(super) enabled: bool,
}

impl Default for CursorHoverSettings {
    fn default() -> Self {
        Self { enabled: true }
    }
}

#[derive(Component)]
pub(super) struct CursorHoverable;

#[derive(Component, Deref)]
pub(crate) struct CursorHover(pub(crate) Vec3);
