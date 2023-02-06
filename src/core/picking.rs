use std::iter;

use bevy::prelude::*;
use bevy_mod_outline::OutlineVolume;
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;

use super::{
    action::Action, city::CityMode, collision_groups::DollisGroups, game_state::GameState,
    object::placing_object, preview::PreviewCamera,
};

pub(super) struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Picked>()
            .add_system(
                Self::ray_system
                    .pipe(Self::picking_system)
                    .pipe(Self::outline_system)
                    .run_if_not(placing_object::placing_active)
                    .run_in_state(GameState::City)
                    .run_not_in_state(CityMode::Lots),
            )
            .add_system(
                Self::ray_system
                    .pipe(Self::picking_system)
                    .pipe(Self::outline_system)
                    .run_in_state(GameState::Family),
            );
    }
}

impl PickingPlugin {
    fn ray_system(
        rapier_ctx: Res<RapierContext>,
        windows: Res<Windows>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
        parents: Query<&Parent>,
        pickable: Query<(), With<Pickable>>,
    ) -> Option<(Entity, Vec3)> {
        if let Some(cursor_pos) = windows
            .get_primary()
            .and_then(|window| window.cursor_position())
        {
            let (&transform, camera) = cameras.single();
            if let Some(ray) = camera.viewport_to_world(&transform, cursor_pos) {
                if let Some((child_entity, toi)) = rapier_ctx.cast_ray(
                    ray.origin,
                    ray.direction,
                    f32::MAX,
                    false,
                    CollisionGroups::new(Group::ALL, Group::OBJECT).into(),
                ) {
                    let picked_entity = iter::once(child_entity)
                        .chain(parents.iter_ancestors(child_entity))
                        .find(|&ancestor_entity| pickable.get(ancestor_entity).is_ok())
                        .expect("entity should have a pickable parent");
                    let position = ray.origin + ray.direction * toi;
                    return Some((picked_entity, position));
                }
            }
        }

        None
    }

    fn picking_system(
        In(pick): In<Option<(Entity, Vec3)>>,
        mut pick_events: EventWriter<Picked>,
        action_state: Res<ActionState<Action>>,
    ) -> Option<Entity> {
        if let Some((entity, position)) = pick {
            if action_state.just_pressed(Action::Confirm) {
                pick_events.send(Picked { entity, position });
                None
            } else {
                Some(entity)
            }
        } else {
            None
        }
    }

    fn outline_system(
        In(hovered_entity): In<Option<Entity>>,
        mut previous_entity: Local<Option<Entity>>,
        mut outlines: Query<&mut OutlineVolume>,
        children: Query<&Children>,
    ) {
        if *previous_entity == hovered_entity {
            return;
        }

        if let Some(hovered_entity) = hovered_entity {
            for child_entity in children.iter_descendants(hovered_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = true;
                }
            }
        }

        if let Some(previous_entity) = *previous_entity {
            for child_entity in children.iter_descendants(previous_entity) {
                if let Ok(mut outline) = outlines.get_mut(child_entity) {
                    outline.visible = false;
                }
            }
        }

        *previous_entity = hovered_entity;
    }
}

#[derive(Component)]
pub(super) struct Pickable;

pub(super) struct Picked {
    pub(super) entity: Entity,
    pub(super) position: Vec3,
}
