use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, RigidBody};
use iyes_loopless::prelude::*;

use super::{city::ActiveCity, game_state::GameState};

pub(super) struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::City, Self::spawn_system);
        app.add_exit_system(GameState::City, Self::despawn_system);
        app.add_enter_system(GameState::Family, Self::spawn_system);
        app.add_exit_system(GameState::Family, Self::despawn_system);
    }
}

impl GroundPlugin {
    fn spawn_system(
        active_cities: Query<Entity, With<ActiveCity>>,
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
    ) {
        const SIZE: f32 = 50.0;
        commands
            .entity(active_cities.single())
            .add_children(|parent| {
                parent
                    .spawn_bundle(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Plane { size: SIZE })),
                        material: materials.add(Color::rgb_u8(69, 108, 69).into()),
                        ..Default::default()
                    })
                    .insert(RigidBody::Fixed)
                    .insert(Collider::cuboid(SIZE / 2.0, 0.0, SIZE / 2.0))
                    .insert(Ground)
                    .insert(Name::new("Ground"));
                parent.spawn_bundle(DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        illuminance: 10000.0,
                        shadows_enabled: true,
                        shadow_depth_bias: 0.25,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(4.0, 8.0, 4.0)
                        .with_rotation(Quat::from_rotation_x(-1.4)),
                    ..Default::default()
                });
            });
    }

    fn despawn_system(
        direction_lights: Query<Entity, With<DirectionalLight>>,
        grounds: Query<Entity, With<Ground>>,
        mut commands: Commands,
    ) {
        commands.entity(direction_lights.single()).despawn();
        commands.entity(grounds.single()).despawn();
    }
}

#[derive(Component)]
pub(super) struct Ground;
