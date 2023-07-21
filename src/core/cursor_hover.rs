use std::iter;

use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_outline::{OutlineBundle, OutlineVolume};
use bevy_rapier3d::prelude::*;

use super::{
    city::CityMode, collision_groups::LifescapeGroupsExt, condition, family::BuildingMode,
    game_state::GameState, object::placing_object::PlacingObject, player_camera::PlayerCamera,
};

pub(super) struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::raycast_system,
                Self::outline_enabling_system,
                Self::outline_disabling_system,
                Self::cleanup_system.run_if(condition::any_component_added::<PlacingObject>()),
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

#[derive(Component, Deref)]
pub(crate) struct CursorHover(pub(crate) Vec3);

pub(super) trait OutlineHoverExt {
    fn hover() -> Self;
}

impl OutlineHoverExt for OutlineBundle {
    fn hover() -> Self {
        Self {
            outline: OutlineVolume {
                visible: false,
                colour: Color::rgba(1.0, 1.0, 1.0, 0.3),
                width: 2.0,
            },
            ..Default::default()
        }
    }
}
