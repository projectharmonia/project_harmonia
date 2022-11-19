use bevy::prelude::{shape::Plane, *};
use bevy_mod_raycast::RayCastMesh;
use bevy_rapier3d::prelude::{Collider, RigidBody};
use iyes_loopless::prelude::*;

use super::{city::ActiveCity, game_state::GameState, picking::Pickable};

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
        commands
            .entity(active_cities.single())
            .add_children(|parent| {
                parent.spawn_bundle(GroundBundle {
                    pbr_bundle: PbrBundle {
                        mesh: meshes.add(Mesh::from(Plane {
                            size: GroundBundle::SIZE,
                        })),
                        material: materials.add(Color::rgb_u8(69, 108, 69).into()),
                        ..Default::default()
                    },
                    ..Default::default()
                });
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

#[derive(Bundle)]
struct GroundBundle {
    name: Name,
    rigid_body: RigidBody,
    collider: Collider,
    ray_cast_mesh: RayCastMesh<Pickable>,
    ground: Ground,
    pickable: Pickable,

    #[bundle]
    pbr_bundle: PbrBundle,
}

impl GroundBundle {
    const SIZE: f32 = 50.0;
}

impl Default for GroundBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Ground"),
            rigid_body: RigidBody::Fixed,
            collider: Collider::cuboid(Self::SIZE / 2.0, 0.0, Self::SIZE / 2.0),
            ray_cast_mesh: RayCastMesh::<Pickable>::default(),
            ground: Ground,
            pickable: Pickable,
            pbr_bundle: Default::default(),
        }
    }
}

#[derive(Component)]
pub(super) struct Ground;
