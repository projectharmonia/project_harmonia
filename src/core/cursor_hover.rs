use std::iter;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_xpbd_3d::prelude::*;

use super::{game_state::GameState, player_camera::PlayerCamera};

pub(super) struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorHoverSettings>().add_systems(
            Update,
            (
                Self::raycast_system
                    .pipe(Self::hover_update_system)
                    .run_if(cursor_hover_enabled),
                Self::cleanup_system
                    .run_if(resource_changed::<CursorHoverSettings>())
                    .run_if(not(cursor_hover_enabled)),
            )
                .run_if(in_state(GameState::City).or_else(in_state(GameState::Family))),
        );
    }
}

impl CursorHoverPlugin {
    fn raycast_system(
        spatial_query: SpatialQuery,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
        parents: Query<&Parent>,
        cursor_hoverable: Query<(), With<CursorHoverable>>,
    ) -> Option<(Entity, Vec3)> {
        let cursor_position = windows.get_single().ok()?.cursor_position()?;
        let (transform, camera) = cameras.single();
        let ray = camera.viewport_to_world(transform, cursor_position)?;
        let hit = spatial_query.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            false,
            Default::default(),
        )?;

        let hovered_entity = iter::once(hit.entity)
            .chain(parents.iter_ancestors(hit.entity))
            .find(|&ancestor_entity| cursor_hoverable.get(ancestor_entity).is_ok())?;

        let position = ray.origin + ray.direction * hit.time_of_impact;
        Some((hovered_entity, position))
    }

    fn hover_update_system(
        In(hit): In<Option<(Entity, Vec3)>>,
        mut commands: Commands,
        cursor_hovers: Query<Entity, With<CursorHover>>,
    ) {
        match (hit, cursor_hovers.get_single().ok()) {
            (Some((hit_entity, position)), None) => {
                commands.entity(hit_entity).insert(CursorHover(position));
            }
            (None, Some(previous_entity)) => {
                commands.entity(previous_entity).remove::<CursorHover>();
            }
            (Some((hit_entity, position)), Some(previous_entity)) => {
                commands.entity(hit_entity).insert(CursorHover(position));
                if hit_entity != previous_entity {
                    commands.entity(previous_entity).remove::<CursorHover>();
                }
            }
            (None, None) => (),
        }
    }

    fn cleanup_system(mut commands: Commands, hovered: Query<Entity, With<CursorHover>>) {
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
