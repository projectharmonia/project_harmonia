pub mod road;

use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::{prelude::*, render::mesh::VertexAttributeValues};
use bevy_atmosphere::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumIter;
use vleue_navigator::prelude::*;

use super::{actor::SelectedActor, WorldState};
use crate::{
    core::GameState,
    game_world::{actor::ACTOR_RADIUS, player_camera::PlayerCamera, Layer},
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
            .add_observer(init)
            .add_observer(activate)
            .add_systems(OnEnter(WorldState::Family), activate_by_actor)
            .add_systems(OnExit(WorldState::City), deactivate.never_param_warn())
            .add_systems(OnExit(WorldState::Family), deactivate.never_param_warn())
            .add_systems(OnExit(GameState::InGame), cleanup);
    }
}

/// City square side size.
const CITY_SIZE: f32 = 500.0;
pub(super) const HALF_CITY_SIZE: f32 = CITY_SIZE / 2.0;

/// Inserts [`TransformBundle`] and places cities next to each other.
fn init(
    trigger: Trigger<OnAdd, City>,
    ground_mesh: Local<GroundMesh>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut placed_citites: ResMut<PlacedCities>,
    mut cities: Query<(&mut Transform, &mut CityNavMesh)>,
) {
    debug!("initializing city `{}`", trigger.entity());
    let (mut transform, mut nav_mesh) = cities.get_mut(trigger.entity()).unwrap();
    transform.translation = Vec3::X * CITY_SIZE * **placed_citites as f32;

    commands.entity(trigger.entity()).with_children(|parent| {
        parent.spawn((
            Ground,
            Mesh3d(ground_mesh.0.clone()),
            MeshMaterial3d::<StandardMaterial>(
                asset_server.load("base/ground/spring_grass/spring_glass.ron"),
            ),
        ));

        nav_mesh.0 = parent
            .spawn((
                ManagedNavMesh::from_id(**placed_citites as u128),
                NavMeshSettings {
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
                Transform::from_rotation(Quat::from_rotation_x(FRAC_PI_2)),
                NavMeshUpdateMode::Direct,
            ))
            .id();
    });

    **placed_citites += 1;
}

fn activate(
    trigger: Trigger<OnAdd, ActiveCity>,
    mut commands: Commands,
    mut active_cities: Query<&mut Visibility>,
) {
    debug!("activating city `{}`", trigger.entity());

    let mut visibility = active_cities.get_mut(trigger.entity()).unwrap();
    *visibility = Visibility::Visible;

    commands.entity(trigger.entity()).with_children(|parent| {
        parent.spawn((
            Sun,
            Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ));
        parent.spawn((PlayerCamera, AtmosphereCamera::default()));
    });
}

fn activate_by_actor(mut commands: Commands, actor_parent: Single<&Parent, With<SelectedActor>>) {
    info!("activating selected actor's city `{}`", ***actor_parent);
    commands.entity(***actor_parent).insert(ActiveCity);
}

fn deactivate(
    mut commands: Commands,
    active_city: Single<(Entity, &mut Visibility), With<ActiveCity>>,
    sun_entity: Single<Entity, With<Sun>>,
    camera_entity: Single<Entity, With<PlayerCamera>>,
) {
    let (city_entity, mut visibility) = active_city.into_inner();
    info!("deactivating city `{city_entity}`");
    *visibility = Visibility::Hidden;
    commands.entity(city_entity).remove::<ActiveCity>();
    commands.entity(*sun_entity).despawn();
    commands.entity(*camera_entity).despawn();
}

/// Removes all cities with their children and resets [`PlacedCities`] counter to 0.
fn cleanup(mut placed_citites: ResMut<PlacedCities>) {
    placed_citites.0 = 0;
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

#[derive(Clone, Component, Copy, Debug, Default, EnumIter, Eq, Hash, PartialEq, SubStates)]
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

#[derive(Component, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
#[require(
    Name,
    Replicated,
    Transform,
    Visibility(|| Visibility::Hidden),
    CityNavMesh(|| CityNavMesh(Entity::PLACEHOLDER)),
    StateScoped<GameState>(|| StateScoped(GameState::InGame)),
)]
pub struct City;

#[derive(Component)]
#[require(City)]
pub struct ActiveCity;

/// Points to assigned navmesh for a city.
#[derive(Component, Deref)]
pub(super) struct CityNavMesh(Entity);

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default, Resource, Deref, DerefMut)]
struct PlacedCities(usize);

#[derive(Component)]
#[require(
    Name(|| Name::new("Ground")),
    Mesh3d,
    MeshMaterial3d<StandardMaterial>,
    Collider(|| Collider::cuboid(CITY_SIZE, 0.0, CITY_SIZE)),
    CollisionLayers(|| CollisionLayers::new(Layer::Ground, LayerMask::ALL)),
)]
pub(super) struct Ground;

#[derive(Component)]
#[require(
    Name(|| Name::new("Sun")),
    DirectionalLight(|| DirectionalLight {
        shadows_enabled: true,
        color: Color::linear_rgb(0.913, 0.855, 0.761),
        ..Default::default()
    }),
)]
struct Sun;
