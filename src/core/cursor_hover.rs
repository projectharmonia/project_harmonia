use std::iter;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_outline::OutlineVolume;
use bevy_rapier3d::prelude::*;

use super::{
    city::CityMode, collision_groups::LifescapeGroupsExt, condition, game_state::GameState,
    object::placing_object::PlacingObject, player_camera::PlayerCamera,
};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone, Copy)]
struct CursorHoverSet;

pub(super) struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            CursorHoverSet
                .run_if(not(any_with_component::<PlacingObject>()))
                .run_if(in_state(GameState::City).or_else(in_state(GameState::Family)))
                .run_if(not(in_state(CityMode::Lots))),
        )
        .add_systems(
            (
                Self::cursor_hover_system,
                Self::outline_enabling_system,
                Self::outline_disabling_system,
                Self::cleanup_system.run_if(condition::any_component_added::<PlacingObject>()),
            )
                .in_set(CursorHoverSet),
        );
    }
}

impl CursorHoverPlugin {
    fn cursor_hover_system(
        mut commands: Commands,
        rapier_ctx: Res<RapierContext>,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
        parents: Query<&Parent>,
        hoverable: Query<(), With<Hoverable>>,
        hovered: Query<Entity, With<CursorHover>>,
    ) {
        let Some(cursor_pos) = windows.single().cursor_position() else {
            return;
        };

        let (&transform, camera) = cameras.single();
        let Some(ray) = camera.viewport_to_world(&transform, cursor_pos) else {
            return;
        };

        if let Some((child_entity, toi)) = rapier_ctx.cast_ray(
            ray.origin,
            ray.direction,
            f32::MAX,
            false,
            CollisionGroups::new(Group::ALL, Group::OBJECT).into(),
        ) {
            let hovered_entity = iter::once(child_entity)
                .chain(parents.iter_ancestors(child_entity))
                .find(|&ancestor_entity| hoverable.get(ancestor_entity).is_ok())
                .expect("entity should have a hoverable parent");
            let position = ray.origin + ray.direction * toi;
            commands
                .entity(hovered_entity)
                .insert(CursorHover(position));
            if let Ok(previous_entity) = hovered.get_single() {
                if hovered_entity != previous_entity {
                    commands.entity(previous_entity).remove::<CursorHover>();
                }
            }
        } else if let Ok(previous_entity) = hovered.get_single() {
            commands.entity(previous_entity).remove::<CursorHover>();
        }
    }

    fn outline_enabling_system(
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
        hovered: Query<Entity, Added<CursorHover>>,
    ) {
        if let Ok(hovered_entity) = hovered.get_single() {
            for child_entity in children.iter_descendants(hovered_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = true;
                }
            }
        }
    }

    fn outline_disabling_system(
        mut unhovered: RemovedComponents<CursorHover>,
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
    ) {
        for parent_entity in &mut unhovered {
            for child_entity in children.iter_descendants(parent_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = false;
                }
            }
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
        hovered: Query<Entity, With<CursorHover>>,
    ) {
        if let Ok(hovered_entity) = hovered.get_single() {
            for child_entity in children.iter_descendants(hovered_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = false;
                }
            }
            commands.entity(hovered_entity).remove::<CursorHover>();
        }
    }
}

#[derive(Component)]
pub(super) struct Hoverable;

#[derive(Component)]
pub(super) struct CursorHover(pub(super) Vec3);
