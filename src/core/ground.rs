use bevy::{
    math::Vec3Swizzles,
    prelude::{shape::Plane, *},
};
use bevy_rapier3d::prelude::*;
use iyes_loopless::prelude::*;

use super::{
    city::ActiveCity, collision_groups::DollisGroups, cursor_hover::Hoverable,
    game_state::GameState, preview::PreviewCamera,
};

pub(super) struct GroundPlugin;

impl Plugin for GroundPlugin {
    fn build(&self, app: &mut App) {
        app.add_enter_system(GameState::City, Self::spawn_system)
            .add_exit_system(GameState::City, Self::despawn_system)
            .add_enter_system(GameState::Family, Self::spawn_system)
            .add_exit_system(GameState::Family, Self::despawn_system);
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
                parent.spawn(GroundBundle {
                    pbr_bundle: PbrBundle {
                        mesh: meshes.add(Mesh::from(Plane {
                            size: GroundBundle::SIZE,
                        })),
                        material: materials.add(Color::rgb_u8(69, 108, 69).into()),
                        ..Default::default()
                    },
                    ..Default::default()
                });
                parent.spawn(DirectionalLightBundle {
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

    /// Converts cursor position into position on the ground.
    pub(super) fn cursor_to_ground_system(
        windows: Res<Windows>,
        cameras: Query<(&GlobalTransform, &Camera), Without<PreviewCamera>>,
    ) -> Option<Vec2> {
        let cursor_position = windows
            .get_primary()
            .and_then(|window| window.cursor_position())?;
        let (&transform, camera) = cameras.single();
        let ray = camera
            .viewport_to_world(&transform, cursor_position)
            .expect("ray should be created from screen coordinates");
        let length = -ray.origin.y / ray.direction.y; // The length to intersect the ground.
        let intersection = ray.origin + ray.direction * length;
        Some(intersection.xz()) // y is always 0.
    }
}

#[derive(Bundle)]
struct GroundBundle {
    name: Name,
    collider: Collider,
    collision_groups: CollisionGroups,
    ground: Ground,
    hoverable: Hoverable,

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
            collider: Collider::cuboid(Self::SIZE / 2.0, 0.0, Self::SIZE / 2.0),
            collision_groups: CollisionGroups::new(Group::GROUND, Group::ALL),
            ground: Ground,
            hoverable: Hoverable,
            pbr_bundle: Default::default(),
        }
    }
}

#[derive(Component)]
pub(super) struct Ground;
