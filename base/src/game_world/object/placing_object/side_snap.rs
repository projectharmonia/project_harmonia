use avian3d::prelude::*;
use bevy::prelude::*;

use super::{PlacingObject, PlacingObjectPlugin};
use crate::game_world::{city::CityMode, family::building::BuildingMode};

pub(super) struct SideSnapPlugin;

impl Plugin for SideSnapPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SideSnap>()
            .add_systems(
                Update,
                (
                    Self::init,
                    Self::snap
                        .after(PlacingObjectPlugin::apply_position)
                        .before(PlacingObjectPlugin::confirm),
                )
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            )
            .add_systems(
                PostUpdate,
                Self::update_nodes
                    .run_if(in_state(CityMode::Objects).or_else(in_state(BuildingMode::Objects))),
            );
    }
}

impl SideSnapPlugin {
    fn init(
        mut commands: Commands,
        objects: Query<Entity, (With<SideSnap>, Without<SideSnapNodes>)>,
    ) {
        for entity in &objects {
            debug!("initializing side snapping for `{entity}`");
            commands.entity(entity).insert(SideSnapNodes::default());
        }
    }

    fn update_nodes(
        mut nodes: Query<&mut SideSnapNodes>,
        objects: Query<(Entity, &SideSnap, Ref<Position>, Ref<Rotation>), Without<PlacingObject>>,
    ) {
        for (entity_a, snap_a, position_a, rotation_a) in &objects {
            if !position_a.is_changed() && !rotation_a.is_changed() {
                continue;
            }

            for (entity_b, &snap_b, position_b, rotation_b) in &objects {
                if entity_a == entity_b {
                    continue;
                }

                if *rotation_a != *rotation_b {
                    continue;
                }

                let disp = **position_b - **position_a;
                if disp.length() - snap_a.distance(snap_b) >= SideSnap::GAP {
                    continue;
                }

                let rotated = **rotation_a * **position_a;
                let other_rotated = **rotation_b * **position_b;
                if rotated.cross(other_rotated).x.is_sign_positive() {
                    connect_nodes(&mut nodes, entity_b, entity_a);
                } else {
                    connect_nodes(&mut nodes, entity_a, entity_b);
                }
            }
        }
    }

    fn snap(
        objects: Query<
            (&SideSnap, &Position, &Rotation, &SideSnapNodes, &Visibility),
            Without<PlacingObject>,
        >,
        mut placing_objects: Query<(&mut Position, &mut Rotation, &SideSnap), With<PlacingObject>>,
    ) {
        let Ok((mut position, mut rotation, snap)) = placing_objects.get_single_mut() else {
            return;
        };

        for (&object_snap, &object_position, &object_rotation, &nodes, visibility) in &objects {
            if visibility == Visibility::Hidden {
                continue;
            }

            let disp = **position - *object_position;
            let distance = snap.distance(object_snap);
            if disp.length() <= distance {
                let dir = disp.normalize();
                let right_dir = *object_rotation * Vec3::X;
                let projection = dir.dot(right_dir);

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
                **position = *object_position + projection.signum() * right_dir * distance;
                *rotation = object_rotation;
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
