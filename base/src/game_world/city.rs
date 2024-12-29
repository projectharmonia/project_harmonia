pub mod road;

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::{prelude::*, render::mesh::VertexAttributeValues};
use bevy_atmosphere::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};
use vleue_navigator::prelude::*;

use super::{
    actor::SelectedActor,
    player_camera::{EnvironmentMap, PlayerCameraBundle},
    WorldState,
};
use crate::{
    asset::collection::Collection,
    core::GameState,
    game_world::{actor::ACTOR_RADIUS, Layer},
};
use road::RoadPlugin;

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(RoadPlugin)
            .add_sub_state::<CityMode>()
            .enable_state_scoped_entities::<CityMode>()
            .register_type::<City>()
            .replicate_group::<(City, Name)>()
            .init_resource::<PlacedCities>()
            .add_systems(OnEnter(WorldState::City), Self::init_activated)
            .add_systems(
                OnEnter(WorldState::Family),
                (Self::activate_by_actor, Self::init_activated).chain(),
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
const CITY_SIZE: f32 = 500.0;
pub(super) const HALF_CITY_SIZE: f32 = CITY_SIZE / 2.0;

impl CityPlugin {
    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn init(
        ground_mesh: Local<GroundMesh>,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, (With<City>, Without<Transform>)>,
    ) {
        for entity in &added_cities {
            debug!("initializing city `{entity}`");

            let navmesh_entity = commands
                .spawn(NavMeshBundle {
                    settings: NavMeshSettings {
                        fixed: Triangulation::from_outer_edges(&[
                            Vec2::new(-HALF_CITY_SIZE, -HALF_CITY_SIZE),
                            Vec2::new(HALF_CITY_SIZE, -HALF_CITY_SIZE),
                            Vec2::new(HALF_CITY_SIZE, HALF_CITY_SIZE),
                            Vec2::new(-HALF_CITY_SIZE, HALF_CITY_SIZE),
                        ]),
                        agent_radius: ACTOR_RADIUS,
                        merge_steps: 1, // Merge triangles when possible to reduce the number of triangles.
                        simplify: 0.01, // Remove points that contribute very little to the mesh.
                        default_search_delta: 0.2, // To avoid agents stuck on namesh edges.
                        ..Default::default()
                    },
                    transform: Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                    update_mode: NavMeshUpdateMode::Direct,
                    ..NavMeshBundle::with_unique_id(placed_citites.0 as u128)
                })
                .id();

            let transform =
                Transform::from_translation(Vec3::X * CITY_SIZE * placed_citites.0 as f32);
            commands
                .entity(entity)
                .insert((
                    StateScoped(GameState::InGame),
                    CityNavMesh(navmesh_entity),
                    SpatialBundle {
                        transform,
                        visibility: Visibility::Hidden,
                        ..Default::default()
                    },
                ))
                .add_child(navmesh_entity)
                .with_children(|parent| {
                    parent.spawn(GroundBundle {
                        pbr_bundle: PbrBundle {
                            mesh: ground_mesh.0.clone(),
                            material: asset_server
                                .load("base/ground/spring_grass/spring_glass.ron"),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
            placed_citites.0 += 1;
        }
    }

    fn activate_by_actor(mut commands: Commands, actors: Query<&Parent, With<SelectedActor>>) {
        let entity = **actors.single();
        info!("activating city `{entity}`");
        commands.entity(entity).insert(ActiveCity);
    }

    fn init_activated(
        mut commands: Commands,
        world_state: Res<State<WorldState>>,
        environment_map: Res<Collection<EnvironmentMap>>,
        mut activated_cities: Query<(Entity, &mut Visibility), Added<ActiveCity>>,
    ) {
        let (entity, mut visibility) = activated_cities.single_mut();
        debug!("initializing activated city `{entity}`");
        *visibility = Visibility::Visible;
        commands.entity(entity).with_children(|parent| {
            parent.spawn((
                Name::new("Sun"),
                StateScoped(**world_state),
                DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        shadows_enabled: true,
                        color: Color::linear_rgb(0.913, 0.855, 0.761),
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ..Default::default()
                },
            ));
            parent.spawn((
                Name::new("Player camera"),
                StateScoped(**world_state),
                PlayerCameraBundle::new(&environment_map),
                AtmosphereCamera::default(),
            ));
        });
    }

    fn deactivate(
        mut commands: Commands,
        mut active_cities: Query<(Entity, &mut Visibility), With<ActiveCity>>,
    ) {
        if let Ok((entity, mut visibility)) = active_cities.get_single_mut() {
            info!("deactivating city `{entity}`");
            *visibility = Visibility::Hidden;
            commands.entity(entity).remove::<ActiveCity>();
        }
    }

    /// Removes all cities with their children and resets [`PlacedCities`] counter to 0.
    fn cleanup(mut placed_citites: ResMut<PlacedCities>) {
        placed_citites.0 = 0;
    }
}

struct GroundMesh(Handle<Mesh>);

impl FromWorld for GroundMesh {
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

        Self(mesh_handle)
    }
}

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, SubStates,
)]
#[source(WorldState = WorldState::City)]
pub enum CityMode {
    #[default]
    Objects,
    Roads,
}

impl CityMode {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "🌳",
            Self::Roads => "🚧",
        }
    }
}

#[derive(Bundle, Default)]
pub struct CityBundle {
    name: Name,
    city: City,
    replication: Replicated,
}

impl CityBundle {
    pub fn new(name: String) -> Self {
        Self {
            name: Name::new(name),
            city: City,
            replication: Replicated,
        }
    }
}

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub struct City;

#[derive(Component)]
pub struct ActiveCity;

/// Points to assigned navmesh for a city.
#[derive(Component, Deref)]
pub(super) struct CityNavMesh(Entity);

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
    pbr_bundle: PbrBundle,
}

impl Default for GroundBundle {
    fn default() -> Self {
        Self {
            name: Name::new("Ground"),
            collider: Collider::cuboid(CITY_SIZE, 0.0, CITY_SIZE),
            collision_layers: CollisionLayers::new(Layer::Ground, LayerMask::ALL),
            ground: Ground,
            pbr_bundle: Default::default(),
        }
    }
}

#[derive(Component)]
pub(super) struct Ground;
