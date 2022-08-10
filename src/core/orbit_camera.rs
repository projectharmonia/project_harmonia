use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{city::City, game_state::GameState};

pub(super) struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::City, Self::spawn_system);
    }
}

impl OrbitCameraPlugin {
    fn spawn_system(
        mut commands: Commands,
        controlled_city: Query<Entity, (With<Visibility>, With<City>)>,
    ) {
        commands
            .entity(controlled_city.single())
            .add_children(|parent| {
                parent.spawn_bundle(Camera3dBundle {
                    transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ..default()
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use std::any;

    use super::*;

    #[test]
    fn spawning() {
        let mut app = App::new();
        app.add_loopless_state(GameState::World)
            .add_plugin(OrbitCameraPlugin);

        let controlled_entity = app
            .world
            .spawn()
            .insert(City)
            .insert(Visibility::default())
            .id();

        app.world.insert_resource(NextState(GameState::City));
        app.update();

        let camera_parent = app
            .world
            .query_filtered::<&Parent, With<Camera>>()
            .single(&app.world);

        assert_eq!(
            camera_parent.get(),
            controlled_entity,
            "Camera should be spawned as a child after inserting {} component",
            any::type_name::<Visibility>()
        );
    }
}
