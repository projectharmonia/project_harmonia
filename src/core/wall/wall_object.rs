use bevy::{prelude::*, render::mesh::VertexAttributeValues, scene::SceneInstanceReady};

use super::{Wall, WallOpening, WallOpenings, WallPlugin};
use crate::core::game_world::WorldName;

pub(super) struct WallObjectPlugin;

impl Plugin for WallObjectPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WallObject>()
            .add_systems(
                Update,
                (Self::openings_update_system, Self::cutout_cleanup_system)
                    .before(WallPlugin::mesh_update_system)
                    .run_if(resource_exists::<WorldName>()),
            )
            .add_systems(
                SpawnScene,
                Self::init_system
                    .run_if(resource_exists::<WorldName>())
                    .after(bevy::scene::scene_spawner_system),
            );
    }
}

impl WallObjectPlugin {
    fn init_system(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        mesh_handles: Query<(Entity, &Handle<Mesh>, &Name)>,
        children: Query<&Children>,
        wall_objects: Query<(Entity, &WallObject), With<WallObject>>,
    ) {
        for event in ready_events.read() {
            let Ok((object_entity, &wall_object)) = wall_objects.get(event.parent) else {
                continue;
            };
            if wall_object != WallObject::Opening {
                continue;
            }

            let (cutout_entity, mesh_handle, _) = mesh_handles
                .iter_many(children.iter_descendants(object_entity))
                .find(|&(.., name)| &**name == "Cutout")
                .expect("openings should contain cutout mesh");

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

    fn openings_update_system(
        mut walls: Query<(Entity, &mut WallOpenings, &Wall)>,
        mut wall_objects: Query<
            (Entity, &GlobalTransform, &mut ObjectCutout),
            Or<(Changed<GlobalTransform>, Added<ObjectCutout>)>,
        >,
    ) {
        for (object_entity, transform, mut cutout) in &mut wall_objects {
            let translation = transform.translation();
            if let Some((wall_entity, mut openings, _)) = walls
                .iter_mut()
                .find(|(.., &wall)| within_wall(wall, translation.xz()))
            {
                if let Some(current_entity) = cutout.wall_entity {
                    if current_entity == wall_entity {
                        openings.update_translation(object_entity, translation)
                    } else {
                        openings.push(WallOpening {
                            object_entity,
                            translation,
                            positions: cutout.positions.clone(),
                        });

                        walls
                            .component_mut::<WallOpenings>(current_entity)
                            .remove_existing(object_entity);

                        cutout.wall_entity = Some(wall_entity);
                    }
                } else {
                    openings.push(WallOpening {
                        object_entity,
                        translation,
                        positions: cutout.positions.clone(),
                    });

                    cutout.wall_entity = Some(wall_entity);
                }
            } else if let Some(surrounding_entity) = cutout.wall_entity.take() {
                walls
                    .component_mut::<WallOpenings>(surrounding_entity)
                    .remove_existing(object_entity);
            }
        }
    }

    fn cutout_cleanup_system(
        mut removed_cutouts: RemovedComponents<ObjectCutout>,
        mut walls: Query<&mut WallOpenings>,
    ) {
        for entity in removed_cutouts.read() {
            for mut openings in &mut walls {
                if let Some(index) = openings
                    .iter()
                    .position(|opening| opening.object_entity == entity)
                {
                    openings.remove(index);
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

/// A component that marks that entity can be placed only on walls or inside them.
#[derive(Component, Reflect, PartialEq, Clone, Copy)]
#[reflect(Component)]
pub(crate) enum WallObject {
    Fixture,
    Opening,
}

// To implement `Reflect`.
impl FromWorld for WallObject {
    fn from_world(_world: &mut World) -> Self {
        Self::Fixture
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
