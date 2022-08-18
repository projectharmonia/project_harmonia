use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{city::City, game_state::GameState};

pub(super) struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::City, Self::spawn_system);
    }
}

impl GroundPlugin {
    fn spawn_system(
        visible_city: Query<Entity, (With<City>, With<Visibility>)>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        commands
            .entity(visible_city.single())
            .add_children(|parent| {
                parent.spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
                    material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                    ..default()
                });
                parent.spawn_bundle(PointLightBundle {
                    point_light: PointLight {
                        intensity: 1500.0,
                        shadows_enabled: true,
                        ..default()
                    },
                    transform: Transform::from_xyz(4.0, 8.0, 4.0),
                    ..default()
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::HeadlessRenderPlugin;

    #[test]
    fn spawning() {
        let mut app = App::new();
        app.add_loopless_state(GameState::World)
            .add_plugin(HeadlessRenderPlugin)
            .add_plugin(GroundPlugin);

        let controlled_entity = app
            .world
            .spawn()
            .insert(City)
            .insert(Visibility::default())
            .id();

        app.world.insert_resource(NextState(GameState::City));
        app.update();

        let ground_parent = app
            .world
            .query_filtered::<&Parent, With<Handle<Mesh>>>()
            .single(&app.world);

        assert_eq!(
            ground_parent.get(),
            controlled_entity,
            "Ground should be spawned as parent",
        );
    }
}
