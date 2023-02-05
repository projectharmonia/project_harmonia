use std::iter;

use bevy::prelude::*;
use bevy_mod_outline::OutlineVolume;
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;

use super::{
    city::CityMode, collision_groups::DollisGroups, game_state::GameState, object::placing_object,
    preview::PreviewCamera,
};

pub(super) struct CursorHoverPlugin;

impl Plugin for CursorHoverPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            Self::cursor_hover_system
                .run_if_not(placing_object::placing_active)
                .run_in_state(GameState::City)
                .run_not_in_state(CityMode::Lots),
        )
        .add_system(
            Self::cursor_hover_system
                .run_if_not(placing_object::placing_active)
                .run_in_state(GameState::Family),
        )
        .add_system(
            Self::outline_enabling_system
                .run_if_not(placing_object::placing_active)
                .run_in_state(GameState::City)
                .run_not_in_state(CityMode::Lots),
        )
        .add_system(
            Self::outline_enabling_system
                .run_if_not(placing_object::placing_active)
                .run_in_state(GameState::Family),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::outline_disabling_system
                .run_if_not(placing_object::placing_active)
                .run_in_state(GameState::City)
                .run_not_in_state(CityMode::Lots),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            Self::outline_disabling_system
                .run_if_not(placing_object::placing_active)
                .run_in_state(GameState::Family),
        );
    }
}

impl CursorHoverPlugin {
    fn cursor_hover_system(
        mut commands: Commands,
        rapier_ctx: Res<RapierContext>,
        windows: Res<Windows>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
        parents: Query<&Parent>,
        hoverable: Query<(), With<Hoverable>>,
        hovered: Query<Entity, With<CursorHover>>,
    ) {
        let Some(cursor_pos) = windows
            .get_primary()
            .and_then(|window| window.cursor_position()) else {
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
        unhovered: RemovedComponents<CursorHover>,
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
    ) {
        for parent_entity in unhovered.iter() {
            for child_entity in children.iter_descendants(parent_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = false;
                }
            }
        }
    }
}

#[derive(Component)]
pub(super) struct Hoverable;

#[derive(Component)]
pub(super) struct CursorHover(pub(super) Vec3);
