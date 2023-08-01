use bevy::{
    prelude::*,
    scene::{self, SceneInstance},
};

pub(super) struct ReadyScenePlugin;

impl Plugin for ReadyScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SceneInstanceReady>().add_systems(
            PostUpdate,
            Self::ready_system.after(scene::scene_spawner_system),
        );
    }
}

impl ReadyScenePlugin {
    fn ready_system(
        mut commands: Commands,
        scene_manager: Res<SceneSpawner>,
        mut ready_events: EventWriter<SceneInstanceReady>,
        scenes: Query<(Entity, &SceneInstance), Without<ReadyScene>>,
    ) {
        for (entity, instance) in scenes.iter() {
            if scene_manager.instance_is_ready(**instance) {
                ready_events.send(SceneInstanceReady { parent: entity });
                commands.entity(entity).insert(ReadyScene);
            }
        }
    }
}

#[derive(Component)]
struct ReadyScene;

#[derive(Event)]
pub(crate) struct SceneInstanceReady {
    pub(crate) parent: Entity,
}
