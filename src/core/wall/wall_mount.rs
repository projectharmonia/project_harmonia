use bevy::{prelude::*, render::mesh::VertexAttributeValues, scene::SceneInstanceReady};

use super::{Aperture, Apertures, Wall, WallPlugin};
use crate::core::{
    game_world::WorldName,
    object::placing_object::{ObjectSnappingSet, PlacingObject},
    wall::wall_mesh::HALF_WIDTH,
};

pub(super) struct WallMountPlugin;

impl Plugin for WallMountPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WallMount>()
            .add_systems(
                Update,
                (
                    (Self::apertures_update_system, Self::cutout_cleanup_system)
                        .before(WallPlugin::mesh_update_system)
                        .run_if(resource_exists::<WorldName>()),
                    Self::wall_snapping_system.in_set(ObjectSnappingSet),
                ),
            )
            .add_systems(
                SpawnScene,
                Self::scene_init_system
                    .run_if(resource_exists::<WorldName>())
                    .after(bevy::scene::scene_spawner_system),
            );
    }
}

impl WallMountPlugin {
    fn scene_init_system(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        mesh_handles: Query<(Entity, &Handle<Mesh>, &Name)>,
        children: Query<&Children>,
        objects: Query<(Entity, &WallMount)>,
    ) {
        for (object_entity, &wall_mount) in
            objects.iter_many(ready_events.read().map(|event| event.parent))
        {
            if wall_mount != WallMount::Embed {
                continue;
            }

            let (cutout_entity, mesh_handle, _) = mesh_handles
                .iter_many(children.iter_descendants(object_entity))
                .find(|&(.., name)| &**name == "Cutout")
                .expect("apertures should contain cutout mesh");

            let mesh = meshes
                .get(mesh_handle)
                .expect("cutout should be loaded when its scene is ready");

            let Some(VertexAttributeValues::Float32x3(positions)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                panic!("cutout should contain vertices positions");
            };

            commands
                .entity(object_entity)
                .insert(ObjectCutout::new(positions));

            commands.entity(cutout_entity).despawn();
        }
    }

    fn apertures_update_system(
        mut walls: Query<(Entity, &mut Apertures, &Wall)>,
        mut objects: Query<
            (Entity, &GlobalTransform, &mut ObjectCutout),
            Or<(Changed<GlobalTransform>, Added<ObjectCutout>)>,
        >,
    ) {
        for (object_entity, transform, mut cutout) in &mut objects {
            let translation = transform.translation();
            if let Some((wall_entity, mut apertures, _)) = walls
                .iter_mut()
                .find(|(.., &wall)| within_wall(wall, translation.xz()))
            {
                if let Some(current_entity) = cutout.wall_entity {
                    if current_entity == wall_entity {
                        apertures.update_translation(object_entity, translation)
                    } else {
                        apertures.push(Aperture {
                            object_entity,
                            translation,
                            positions: cutout.positions.clone(),
                        });

                        walls
                            .component_mut::<Apertures>(current_entity)
                            .remove_existing(object_entity);

                        cutout.wall_entity = Some(wall_entity);
                    }
                } else {
                    apertures.push(Aperture {
                        object_entity,
                        translation,
                        positions: cutout.positions.clone(),
                    });

                    cutout.wall_entity = Some(wall_entity);
                }
            } else if let Some(surrounding_entity) = cutout.wall_entity.take() {
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
                WallMount::Embed => Vec2::ZERO,
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

    fn cutout_cleanup_system(
        mut removed_cutouts: RemovedComponents<ObjectCutout>,
        mut walls: Query<&mut Apertures>,
    ) {
        for entity in removed_cutouts.read() {
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
#[derive(Component, Reflect, PartialEq, Clone, Copy)]
#[reflect(Component)]
pub(crate) enum WallMount {
    Attach,
    Embed,
}

// To implement `Reflect`.
impl FromWorld for WallMount {
    fn from_world(_world: &mut World) -> Self {
        Self::Attach
    }
}

#[derive(Component, Default)]
struct ObjectCutout {
    positions: Vec<Vec3>,
    wall_entity: Option<Entity>,
}

impl ObjectCutout {
    fn new(positions: &[[f32; 3]]) -> Self {
        Self {
            positions: positions.iter().copied().map(From::from).collect(),
            wall_entity: Default::default(),
        }
    }
}
