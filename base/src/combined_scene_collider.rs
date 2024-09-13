use avian3d::prelude::*;
use bevy::{
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::InheritOutlineBundle;

use crate::core::GameState;

pub(super) struct CombinedSceneColliderPlugin;

impl Plugin for CombinedSceneColliderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
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
        scenes: Query<Entity, With<CombinedSceneCollider>>,
        chidlren: Query<&Children>,
        child_meshes: Query<(&Transform, &Handle<Mesh>)>,
    ) {
        for scene_entity in scenes.iter_many(ready_events.read().map(|event| event.parent)) {
            let mut merged_mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_inserted_indices(Indices::U32(Vec::new()));

            for child_entity in chidlren.iter_descendants(scene_entity) {
                commands
                    .entity(child_entity)
                    .insert(InheritOutlineBundle::default());

                if let Ok((&transform, mesh_handle)) = child_meshes.get(child_entity) {
                    let mut mesh = meshes
                        .get(mesh_handle)
                        .cloned()
                        .expect("scene mesh should always be valid");
                    mesh.transform_by(transform);
                    merged_mesh.merge(&mesh);
                }
            }

            let collider = Collider::convex_hull_from_mesh(&merged_mesh)
                .expect("object mesh should be in compatible format");

            debug!("inserting collider for `{scene_entity}`");
            commands.entity(scene_entity).insert(collider);
        }
    }
}

#[derive(Component)]
pub(super) struct CombinedSceneCollider;
