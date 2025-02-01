use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, MeshAabb},
        render_resource::PrimitiveTopology,
    },
    scene::SceneInstanceReady,
};

pub(super) struct SceneColliderConstructorPlugin;

impl Plugin for SceneColliderConstructorPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SceneColliderConstructor>()
            .add_observer(init);
    }
}

fn init(
    trigger: Trigger<SceneInstanceReady>,
    meshes: Res<Assets<Mesh>>,
    mut scenes: Query<(&Children, &SceneColliderConstructor, &mut Collider)>,
    scene_meshes: Query<(&Transform, Option<&Mesh3d>, Option<&Children>)>,
) {
    let Ok((children, constructor, mut collider)) = scenes.get_mut(trigger.entity()) else {
        return;
    };

    debug!("generating collider for scene `{}`", trigger.entity());

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

    *collider = match constructor {
        SceneColliderConstructor::Aabb => {
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
        SceneColliderConstructor::ConvexHull => Collider::convex_hull_from_mesh(&combined_mesh)
            .expect("object mesh should be in compatible format"),
    };
}

fn recursive_merge(
    meshes: &Assets<Mesh>,
    scene_meshes: &Query<(&Transform, Option<&Mesh3d>, Option<&Children>)>,
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
#[require(Collider)]
pub(super) enum SceneColliderConstructor {
    Aabb,
    ConvexHull,
}
