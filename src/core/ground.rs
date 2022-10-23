use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, RigidBody};
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
        visible_cities: Query<Entity, (With<City>, With<Visibility>)>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        const SIZE: f32 = 5.0;
        commands
            .entity(visible_cities.single())
            .add_children(|parent| {
                parent
                    .spawn_bundle(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Plane { size: SIZE })),
                        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                        ..Default::default()
                    })
                    .insert(RigidBody::Fixed)
                    .insert(Collider::cuboid(SIZE / 2.0, 0.0, SIZE / 2.0));
                parent.spawn_bundle(PointLightBundle {
                    point_light: PointLight {
                        intensity: 1500.0,
                        shadows_enabled: true,
                        shadow_depth_bias: 0.25,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(4.0, 8.0, 4.0),
                    ..Default::default()
                });
            });
    }
}
