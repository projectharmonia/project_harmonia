use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    scene::{self, SceneInstanceReady},
};

use crate::core::GameState;

pub(super) struct CombinedSceneColliderPlugin;

impl Plugin for CombinedSceneColliderPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CombinedSceneCollider>().add_systems(
            SpawnScene,
            Self::init
                .run_if(in_state(GameState::InGame))
                .after(scene::scene_spawner_system),
        );
    }
}

impl CombinedSceneColliderPlugin {
    fn init(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        scenes: Query<(Entity, &Children, &CombinedSceneCollider)>,
        scene_meshes: Query<(&Transform, Option<&Handle<Mesh>>, Option<&Children>)>,
    ) {
        for (scene_entity, children, combined_collider) in
            scenes.iter_many(ready_events.read().map(|event| event.parent))
        {
            let mut combined_mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_inserted_indices(Indices::U32(Vec::new()));

            for &child_entity in children {
                recursive_merge(
                    &meshes,
                    &scene_meshes,
                    child_entity,
                    Default::default(),
                    &mut combined_mesh,
                );
            }

            let collider = match combined_collider {
                CombinedSceneCollider::Aabb => {
                    let aabb = combined_mesh
                        .compute_aabb()
                        .expect("object mesh should be in compatible format");
                    let center: Vec3 = aabb.center.into();
                    let cuboid = Collider::cuboid(
                        aabb.half_extents.x * 2.0,
                        aabb.half_extents.y * 2.0,
                        aabb.half_extents.z * 2.0,
                    );
                    Collider::compound(vec![(center, Rotation::default(), cuboid)])
                }
                CombinedSceneCollider::ConvexHull => {
                    Collider::convex_hull_from_mesh(&combined_mesh)
                        .expect("object mesh should be in compatible format")
                }
            };

            debug!("inserting collider for `{scene_entity}`");
            commands.entity(scene_entity).insert(collider);
        }
    }
}

fn recursive_merge(
    meshes: &Assets<Mesh>,
    scene_meshes: &Query<(&Transform, Option<&Handle<Mesh>>, Option<&Children>)>,
    current_entity: Entity,
    parent_transform: Transform,
    combined_mesh: &mut Mesh,
) {
    let (transform, mesh_handle, children) = scene_meshes
        .get(current_entity)
        .expect("all scene children should have transform");

    let current_transform = parent_transform * *transform;
    if let Some(mesh_handle) = mesh_handle {
        let mut mesh = meshes
            .get(mesh_handle)
            .cloned()
            .expect("all scene children should be loaded");
        mesh.transform_by(current_transform);
        combined_mesh.merge(&mesh);
    }

    if let Some(children) = children {
        for &child in children {
            recursive_merge(
                meshes,
                scene_meshes,
                child,
                current_transform,
                combined_mesh,
            );
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(super) enum CombinedSceneCollider {
    Aabb,
    ConvexHull,
}
