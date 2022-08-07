use bevy::prelude::*;

use super::game_world::Control;

pub(super) struct OrbitCameraPlugin;

impl Plugin for OrbitCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::spawn_test_world);
    }
}

impl OrbitCameraPlugin {
    fn spawn_test_world(
        controlled_entities: Query<Entity, Added<Control>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        for entity in controlled_entities.iter() {
            commands.entity(entity).add_children(|parent| {
                parent.spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
                    material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                    ..default()
                });
                parent.spawn_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                    material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
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
                parent.spawn_bundle(Camera3dBundle {
                    transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ..default()
                });
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::any;

    use crate::core::tests::HeadlessRenderPlugin;

    use super::*;

    #[test]
    fn update() {
        let mut app = App::new();
        app.add_plugin(HeadlessRenderPlugin)
            .add_plugin(OrbitCameraPlugin);

        let controlled_entity = app.world.spawn().insert(Control).id();

        app.update();

        let camera_parent = app
            .world
            .query_filtered::<&Parent, With<Camera>>()
            .single(&app.world);
        assert_eq!(
            camera_parent.get(),
            controlled_entity,
            "Camera should be spawned after inserting {} component as a child",
            any::type_name::<Camera>()
        );
    }
}
