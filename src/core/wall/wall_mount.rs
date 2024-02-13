use bevy::prelude::*;
use bevy_xpbd_3d::prelude::*;

use super::{Aperture, Apertures, Wall, WallPlugin};
use crate::core::{
    game_world::WorldName,
    object::placing_object::{ObjectSnappingSet, PlacingObject},
    wall::wall_mesh::HALF_WIDTH,
    Layer,
};

pub(super) struct WallMountPlugin;

impl Plugin for WallMountPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec2>()
            .register_type::<Vec<Vec2>>()
            .register_type::<WallMount>()
            .add_systems(Update, Self::wall_snapping_system.in_set(ObjectSnappingSet))
            .add_systems(
                SpawnScene,
                Self::scene_init_system.run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::post_scene_init_system,
                    (Self::apertures_update_system, Self::cleanup_system)
                        .before(WallPlugin::mesh_update_system),
                )
                    .run_if(resource_exists::<WorldName>()),
            );
    }
}

impl WallMountPlugin {
    fn scene_init_system(mut commands: Commands, mut objects: Query<Entity, Added<WallMount>>) {
        for entity in &mut objects {
            commands.entity(entity).insert(ObjectWall::default());
        }
    }

    fn post_scene_init_system(
        mut objects: Query<&mut CollisionLayers, (Added<CollisionLayers>, With<WallMount>)>,
    ) {
        for mut collision_layers in &mut objects {
            *collision_layers = collision_layers.remove_mask(Layer::Wall);
        }
    }

    fn apertures_update_system(
        mut walls: Query<(Entity, &mut Apertures, &Wall)>,
        mut wall_mounts: Query<
            (Entity, &GlobalTransform, &WallMount, &mut ObjectWall),
            Changed<GlobalTransform>,
        >,
    ) {
        for (object_entity, transform, wall_mount, mut object_wall) in &mut wall_mounts {
            let WallMount::Embed(positions) = wall_mount else {
                continue;
            };

            let translation = transform.translation();
            if let Some((wall_entity, mut apertures, _)) = walls
                .iter_mut()
                .find(|(.., &wall)| within_wall(wall, translation.xz()))
            {
                if let Some(current_entity) = object_wall.0 {
                    if current_entity == wall_entity {
                        apertures.update_translation(object_entity, translation)
                    } else {
                        apertures.push(Aperture {
                            object_entity,
                            translation,
                            positions: positions.clone(),
                        });

                        walls
                            .component_mut::<Apertures>(current_entity)
                            .remove_existing(object_entity);

                        object_wall.0 = Some(wall_entity);
                    }
                } else {
                    apertures.push(Aperture {
                        object_entity,
                        translation,
                        positions: positions.clone(),
                    });

                    object_wall.0 = Some(wall_entity);
                }
            } else if let Some(surrounding_entity) = object_wall.0.take() {
                walls
                    .component_mut::<Apertures>(surrounding_entity)
                    .remove_existing(object_entity);
            }
        }
    }

    fn wall_snapping_system(
        walls: Query<&Wall>,
        mut placing_objects: Query<(&mut Transform, &mut PlacingObject, &WallMount)>,
    ) {
        let Ok((mut transform, mut placing_object, wall_mount)) = placing_objects.get_single_mut()
        else {
            return;
        };

        const SNAP_DELTA: f32 = 1.0;
        let translation_2d = transform.translation.xz();
        if let Some((dir, wall_point)) = walls
            .iter()
            .map(|wall| {
                let dir = wall.dir();
                (dir, closest_point(wall.start, dir, translation_2d))
            })
            .find(|(_, point)| point.distance(translation_2d) <= SNAP_DELTA)
        {
            const GAP: f32 = 0.03; // A small gap between the object and wall to avoid collision.
            let sign = dir.perp_dot(translation_2d - wall_point).signum();
            let offset = match wall_mount {
                WallMount::Embed(_) => Vec2::ZERO,
                WallMount::Attach => sign * dir.perp().normalize() * (HALF_WIDTH + GAP),
            };
            let snap_point = wall_point + offset;
            let angle = dir.angle_between(Vec2::X * sign);
            transform.translation.x = snap_point.x;
            transform.translation.z = snap_point.y;
            transform.rotation = Quat::from_rotation_y(angle);
            if !placing_object.allowed_place {
                placing_object.allowed_place = true;
            }
        } else if placing_object.allowed_place {
            placing_object.allowed_place = false;
        }
    }

    fn cleanup_system(
        mut removed_objects: RemovedComponents<ObjectWall>,
        mut walls: Query<&mut Apertures>,
    ) {
        for entity in removed_objects.read() {
            for mut apertures in &mut walls {
                if let Some(index) = apertures
                    .iter()
                    .position(|aperture| aperture.object_entity == entity)
                {
                    apertures.remove(index);
                }
            }
        }
    }
}

/// Returns `true` if a point belongs to a wall.
fn within_wall(wall: Wall, point: Vec2) -> bool {
    let wall_dir = wall.end - wall.start;
    let point_dir = point - wall.start;
    if wall_dir.perp_dot(point_dir).abs() > 0.1 {
        return false;
    }

    let dot = wall_dir.dot(point_dir);
    if dot < 0.0 {
        return false;
    }

    dot <= wall_dir.length_squared()
}

/// Returns the minimal distance from point `p` to the segment defined by its `origin` and `displacement` vector.
fn closest_point(origin: Vec2, displacement: Vec2, p: Vec2) -> Vec2 {
    // Consider the line extending the segment, parameterized as `origin + t * displacement`.
    let t = (p - origin).dot(displacement) / displacement.length_squared();
    // We clamp `t` to handle points outside the segment.
    origin + t.clamp(0.0, 1.0) * displacement // Projection of point `p` onto the segment.
}

/// A component that marks that entity can be placed only on walls or inside them.
#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) enum WallMount {
    Attach,
    Embed(Vec<Vec2>),
}

// To implement `Reflect`.
impl FromWorld for WallMount {
    fn from_world(_world: &mut World) -> Self {
        Self::Attach
    }
}

#[derive(Component, Default)]
struct ObjectWall(Option<Entity>);
