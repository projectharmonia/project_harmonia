use bevy::prelude::*;

use super::PlacingObject;
use crate::game_world::{city::CityMode, family::building::BuildingMode};

pub(super) struct SideSnapPlugin;

impl Plugin for SideSnapPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SideSnap>()
            .add_systems(
                Update,
                Self::snap
                    .never_param_warn()
                    .after(super::apply_position)
                    .run_if(in_state(CityMode::Objects).or(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                PostUpdate,
                Self::update_nodes
                    .run_if(in_state(CityMode::Objects).or(in_state(BuildingMode::Objects))),
            );
    }
}

impl SideSnapPlugin {
    fn update_nodes(
        mut nodes: Query<&mut SideSnapNodes>,
        objects: Query<(Entity, &SideSnap, Ref<Transform>), Without<PlacingObject>>,
    ) {
        for (entity_a, snap_a, transform_a) in &objects {
            if !transform_a.is_changed() {
                continue;
            }

            for (entity_b, &snap_b, transform_b) in &objects {
                if entity_a == entity_b {
                    continue;
                }

                if transform_a.rotation != transform_b.rotation {
                    continue;
                }

                let disp = transform_b.translation - transform_a.translation;
                if disp.length() - snap_a.distance(snap_b) >= SideSnap::GAP {
                    continue;
                }

                let rotated = transform_a.rotation * transform_a.translation;
                let other_rotated = transform_b.rotation * transform_b.translation;
                if rotated.cross(other_rotated).x.is_sign_positive() {
                    connect_nodes(&mut nodes, entity_b, entity_a);
                } else {
                    connect_nodes(&mut nodes, entity_a, entity_b);
                }
            }
        }
    }

    fn snap(
        placing_object: Single<(&mut Transform, &SideSnap), With<PlacingObject>>,
        objects: Query<
            (&SideSnap, &Transform, &SideSnapNodes, &Visibility),
            Without<PlacingObject>,
        >,
    ) {
        let (mut transform, snap) = placing_object.into_inner();
        for (&object_snap, &object_transform, &nodes, visibility) in &objects {
            if visibility == Visibility::Hidden {
                continue;
            }

            let disp = transform.translation - object_transform.translation;
            let distance = snap.distance(object_snap);
            if disp.length() <= distance {
                let dir = disp.normalize();
                let right_dir = object_transform.right();
                let projection = dir.dot(*right_dir);

                if projection.is_sign_positive() {
                    if nodes.left_entity.is_some() {
                        trace!("ignoring snapping because left side is already snapped");
                        continue;
                    }
                } else if nodes.right_entity.is_some() {
                    trace!("ignoring snapping because right side is already snapped");
                    continue;
                }

                trace!("applying snapping");
                transform.translation =
                    object_transform.translation + projection.signum() * right_dir * distance;
                transform.rotation = object_transform.rotation;
                return;
            }
        }
    }
}

fn connect_nodes(nodes: &mut Query<&mut SideSnapNodes>, left_entity: Entity, right_entity: Entity) {
    debug!("connecting `{left_entity}` with `{right_entity}`");
    let mut left_nodes = nodes
        .get_mut(left_entity)
        .expect("left side snap entity should have nodes");
    left_nodes.right_entity = Some(right_entity);

    let mut right_nodes = nodes
        .get_mut(right_entity)
        .expect("right side snap entity should have nodes");
    right_nodes.left_entity = Some(left_entity);
}

/// Enables attaching objects to other objects.
#[derive(Component, Reflect, Clone, Copy, Deref)]
#[reflect(Component)]
#[require(SideSnapNodes)]
pub(crate) struct SideSnap {
    half_width: f32,
}

impl SideSnap {
    /// Small gap to avoid collision detection.
    const GAP: f32 = 0.00001;

    fn distance(self, other: Self) -> f32 {
        self.half_width + other.half_width + Self::GAP
    }
}

#[derive(Component, Default, Clone, Copy)]
struct SideSnapNodes {
    left_entity: Option<Entity>,
    right_entity: Option<Entity>,
}
