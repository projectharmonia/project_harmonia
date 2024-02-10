use std::iter;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_rapier3d::prelude::*;

use super::{
    city::CityMode, collision_groups::HarmoniaGroupsExt, family::BuildingMode,
    game_state::GameState, player_camera::PlayerCamera,
};

pub(super) struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorHoverSettings>().add_systems(
            Update,
            (
                Self::raycast_system.run_if(cursor_hover_enabled),
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
        mut commands: Commands,
        rapier_ctx: Res<RapierContext>,
        building_mode: Res<State<BuildingMode>>,
        city_mode: Res<State<CityMode>>,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
        parents: Query<&Parent>,
        cursor_hoverable: Query<(), With<CursorHoverable>>,
        cursor_hovers: Query<Entity, With<CursorHover>>,
    ) {
        let Some(cursor_pos) = windows.single().cursor_position() else {
            return;
        };

        let (&transform, camera) = cameras.single();
        let Some(ray) = camera.viewport_to_world(&transform, cursor_pos) else {
            return;
        };

        let mut groups = Group::GROUND | Group::ACTOR;
        if *building_mode != BuildingMode::Walls || *city_mode != CityMode::Lots {
            groups |= Group::OBJECT;
        }

        if let Some((child_entity, toi)) = rapier_ctx.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            false,
            CollisionGroups::new(Group::ALL, groups).into(),
        ) {
            let hovered_entity = iter::once(child_entity)
                .chain(parents.iter_ancestors(child_entity))
                .find(|&ancestor_entity| cursor_hoverable.get(ancestor_entity).is_ok())
                .expect("entity should have a hoverable parent");
            let position = ray.origin + ray.direction * toi;
            commands
                .entity(hovered_entity)
                .insert(CursorHover(position));
            if let Ok(previous_entity) = cursor_hovers.get_single() {
                if hovered_entity != previous_entity {
                    commands.entity(previous_entity).remove::<CursorHover>();
                }
            }
        } else if let Ok(previous_entity) = cursor_hovers.get_single() {
            commands.entity(previous_entity).remove::<CursorHover>();
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
