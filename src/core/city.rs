use bevy::{prelude::*, scene};
use bevy_atmosphere::prelude::*;
use bevy_replicon::prelude::*;
use derive_more::Display;
use strum::EnumIter;

use super::{
    actor::ActiveActor,
    game_state::GameState,
    game_world::WorldName,
    player_camera::{PlayerCamera, PlayerCameraBundle},
};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<CityMode>()
            .replicate::<City>()
            .not_replicate_if_present::<Transform, City>()
            .init_resource::<PlacedCities>()
            .add_systems(OnEnter(GameState::City), Self::activation_system)
            .add_systems(
                OnEnter(GameState::Family),
                (
                    Self::actor_activation_system,
                    apply_deferred,
                    Self::activation_system,
                )
                    .chain(),
            )
            .add_systems(OnExit(GameState::City), Self::deactivation_system)
            .add_systems(OnExit(GameState::Family), Self::deactivation_system)
            .add_systems(
                Update,
                (
                    Self::init_system
                        .after(scene::scene_spawner_system)
                        .run_if(resource_exists::<WorldName>()),
                    Self::cleanup_system.run_if(resource_removed::<WorldName>()),
                ),
            );
    }
}

/// City square side size.
const CITY_SIZE: f32 = 100.0;
pub(super) const HALF_CITY_SIZE: f32 = CITY_SIZE / 2.0;

impl CityPlugin {
    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn init_system(
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, Added<City>>,
    ) {
        for entity in &added_cities {
            let transform =
                Transform::from_translation(Vec3::X * CITY_SIZE * placed_citites.0 as f32);
            commands.entity(entity).insert((
                TransformBundle::from_transform(transform),
                VisibilityBundle {
                    visibility: Visibility::Hidden,
                    ..Default::default()
                },
            ));
            placed_citites.0 += 1;
        }
    }

    fn actor_activation_system(
        mut commands: Commands,
        activated_actors: Query<&Parent, With<ActiveActor>>,
    ) {
        commands
            .entity(activated_actors.single().get())
            .insert(ActiveCity);
    }

    pub(super) fn activation_system(
        mut commands: Commands,
        mut activated_cities: Query<(Entity, &mut Visibility), Added<ActiveCity>>,
    ) {
        let (entity, mut visibility) = activated_cities.single_mut();
        *visibility = Visibility::Visible;
        commands.entity(entity).with_children(|parent| {
            parent.spawn((PlayerCameraBundle::default(), AtmosphereCamera::default()));
        });
    }

    fn deactivation_system(
        mut commands: Commands,
        mut active_cities: Query<(Entity, &mut Visibility), With<ActiveCity>>,
        cameras: Query<Entity, With<PlayerCamera>>,
    ) {
        let (entity, mut visibility) = active_cities.single_mut();
        *visibility = Visibility::Hidden;
        commands.entity(entity).remove::<ActiveCity>();
        commands.entity(cameras.single()).despawn();
    }

    /// Removes all cities with their children and resets [`PlacedCities`] counter to 0.
    fn cleanup_system(
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

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, States,
)]
pub(crate) enum CityMode {
    #[default]
    Objects,
    Lots,
}

impl CityMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "🌳",
            Self::Lots => "🚧",
        }
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    replication: Replication,
}

impl CityBundle {
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            city: City,
            replication: Replication,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct City;

#[derive(Component)]
pub(crate) struct ActiveCity;

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default, Resource)]
struct PlacedCities(usize);
