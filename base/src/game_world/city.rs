use bevy::{prelude::*, render::mesh::VertexAttributeValues};
use bevy_atmosphere::prelude::*;
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use oxidized_navigation::NavMeshAffector;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::{
    actor::SelectedActor,
    hover::Hoverable,
    player_camera::{PlayerCamera, PlayerCameraBundle},
    WorldState,
};
use crate::{core::GameState, game_world::Layer};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<CityMode>()
            .enable_state_scoped_entities::<CityMode>()
            .register_type::<City>()
            .replicate::<City>()
            .init_resource::<PlacedCities>()
            .add_systems(OnEnter(WorldState::City), Self::init_activated)
            .add_systems(
                OnEnter(WorldState::Family),
                (Self::activate, Self::init_activated).chain(),
            )
            .add_systems(OnExit(WorldState::City), Self::deactivate)
            .add_systems(OnExit(WorldState::Family), Self::deactivate)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(OnExit(GameState::InGame), Self::cleanup);
    }
}

/// City square side size.
const CITY_SIZE: f32 = 100.0;
pub const HALF_CITY_SIZE: f32 = CITY_SIZE / 2.0;

impl CityPlugin {
    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn init(
        ground_scene: Local<GroundScene>,
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, Added<City>>,
    ) {
        for entity in &added_cities {
            debug!("initializing city `{entity:?}`");

            let transform =
                Transform::from_translation(Vec3::X * CITY_SIZE * placed_citites.0 as f32);
            commands
                .entity(entity)
                .insert((
                    TransformBundle::from_transform(transform),
                    VisibilityBundle {
                        visibility: Visibility::Hidden,
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    parent.spawn(GroundBundle {
                        pbr_bundle: PbrBundle {
                            mesh: ground_scene.mesh_handle.clone(),
                            material: ground_scene.material_handle.clone(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
            placed_citites.0 += 1;
        }
    }

    fn activate(mut commands: Commands, actors: Query<&Parent, With<SelectedActor>>) {
        let entity = actors.single().get();
        info!("activating city `{entity:?}`");
        commands.entity(entity).insert(ActiveCity);
    }

    fn init_activated(
        mut commands: Commands,
        mut activated_cities: Query<(Entity, &mut Visibility), Added<ActiveCity>>,
    ) {
        let (entity, mut visibility) = activated_cities.single_mut();
        debug!("initializing activated city `{entity:?}`");
        *visibility = Visibility::Visible;
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Sun,
                DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        shadows_enabled: true,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ..Default::default()
                },
            ));
            parent.spawn((PlayerCameraBundle::default(), AtmosphereCamera::default()));
        });
    }

    fn deactivate(
        mut commands: Commands,
        mut active_cities: Query<(Entity, &mut Visibility), With<ActiveCity>>,
        cameras: Query<Entity, With<PlayerCamera>>,
        lights: Query<Entity, With<Sun>>,
    ) {
        if let Ok((entity, mut visibility)) = active_cities.get_single_mut() {
            info!("deactivating city `{entity:?}`");
            *visibility = Visibility::Hidden;
            commands.entity(entity).remove::<ActiveCity>();
            commands.entity(cameras.single()).despawn();
            commands.entity(lights.single()).despawn();
        }
    }

    /// Removes all cities with their children and resets [`PlacedCities`] counter to 0.
    fn cleanup(
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        cities: Query<Entity, With<City>>,
    ) {
        placed_citites.0 = 0;
        for entity in &cities {
            commands.entity(entity).despawn_recursive();
        }
    }
}

struct GroundScene {
    mesh_handle: Handle<Mesh>,
    material_handle: Handle<StandardMaterial>,
}

impl FromWorld for GroundScene {
    fn from_world(world: &mut World) -> Self {
        let mut mesh = Plane3d::default().mesh().size(CITY_SIZE, CITY_SIZE).build();
        let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
        else {
            panic!("generated plane should have UVs");
        };

        // Adjust UVs to tile the texture properly.
        for point in uvs {
            for value in point {
                *value *= CITY_SIZE;
            }
        }

        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes.add(mesh);

        let asset_server = world.resource::<AssetServer>();
        let material = StandardMaterial {
            base_color_texture: Some(
                asset_server.load("base/ground/spring_grass/spring_grass_base_color.png"),
            ),
            perceptual_roughness: 0.0,
            reflectance: 0.0,
            ..Default::default()
        };

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        let material_handle = materials.add(material);

        Self {
            mesh_handle,
            material_handle,
        }
    }
}

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, SubStates,
)]
#[source(WorldState = WorldState::City)]
pub enum CityMode {
    #[default]
    Objects,
    Lots,
}

impl CityMode {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "🌳",
            Self::Lots => "🚧",
        }
    }
}

#[derive(Bundle, Default)]
pub struct CityBundle {
    city: City,
    replication: Replicated,
}

impl CityBundle {
    pub fn new(name: String) -> Self {
        Self {
            city: City { name },
            replication: Replicated,
        }
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct City {
    pub name: String,
}

#[derive(Component)]
pub struct ActiveCity;

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default, Resource)]
struct PlacedCities(usize);

#[derive(Bundle)]
struct GroundBundle {
    name: Name,
    collider: Collider,
    collision_layers: CollisionLayers,
    ground: Ground,
    cursor_hoverable: Hoverable,
    nav_mesh_affector: NavMeshAffector,
    pbr_bundle: PbrBundle,
}

impl Default for GroundBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Ground"),
            collider: Collider::cuboid(CITY_SIZE, 0.0, CITY_SIZE),
            collision_layers: CollisionLayers::new(LayerMask::ALL, Layer::Ground),
            ground: Ground,
            cursor_hoverable: Hoverable,
            nav_mesh_affector: NavMeshAffector,
            pbr_bundle: Default::default(),
        }
    }
}

#[derive(Component)]
pub(super) struct Ground;

#[derive(Component)]
struct Sun;
