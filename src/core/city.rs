use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{
    game_state::GameState,
    game_world::{GameEntity, GameWorld},
    orbit_camera::OrbitCameraBundle,
};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedCities>()
            .register_type::<City>()
            .add_enter_system(GameState::City, Self::camera_spawn_system)
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>())
            .add_system(Self::placement_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::placed_cities_reset_system.run_if_resource_removed::<GameWorld>());
    }
}

impl CityPlugin {
    const CITY_SIZE: f32 = 100.0;

    fn camera_spawn_system(
        mut commands: Commands,
        visible_cities: Query<Entity, (With<Visibility>, With<City>)>,
    ) {
        commands
            .entity(visible_cities.single())
            .with_children(|parent| {
                parent.spawn_bundle(OrbitCameraBundle::default());
            });
    }

    /// Removes all cities and their children.
    fn cleanup_system(mut commands: Commands, cities: Query<Entity, With<City>>) {
        for entity in &cities {
            commands.entity(entity).despawn_recursive();
        }
    }

    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn placement_system(
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, Added<City>>,
    ) {
        for entity in &added_cities {
            let transform =
                Transform::from_translation(Vec3::X * Self::CITY_SIZE * placed_citites.0 as f32);
            commands
                .entity(entity)
                .insert_bundle(TransformBundle::from_transform(transform));
            placed_citites.0 += 1;
        }
    }

    /// Resets [`PlacedCities`] counter to 0.
    fn placed_cities_reset_system(mut placed_citites: ResMut<PlacedCities>) {
        placed_citites.0 = 0;
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    game_world: GameEntity,
}

impl CityBundle {
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            city: City,
            game_world: GameEntity,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct City;

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default)]
struct PlacedCities(usize);
