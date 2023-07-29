use bevy::{
    prelude::*,
    scene::{self, SceneInstance},
};

pub(super) struct ReadyScenePlugin;

impl Plugin for ReadyScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            Self::ready_system.after(scene::scene_spawner_system),
        );
    }
}

impl ReadyScenePlugin {
    fn ready_system(
        mut commands: Commands,
        scene_manager: Res<SceneSpawner>,
        scenes: Query<(Entity, &SceneInstance), Without<ReadyScene>>,
    ) {
        for (entity, instance) in scenes.iter() {
            if scene_manager.instance_is_ready(**instance) {
                commands.entity(entity).insert(ReadyScene);
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct ReadyScene;
