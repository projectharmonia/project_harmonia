use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{
    game_state::GameState,
    game_world::{GameEntity, GameWorld},
    orbit_camera::{OrbitCameraBundle, OrbitOrigin},
    settings::Settings,
};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedCities>()
            .register_type::<City>()
            .add_enter_system(GameState::City, Self::enter_system)
            .add_exit_system(
                GameState::City,
                Self::exit_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>())
            .add_system(Self::placement_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::placed_cities_reset_system.run_if_resource_removed::<GameWorld>());
    }
}

impl CityPlugin {
    const CITY_SIZE: f32 = 100.0;

    fn enter_system(
        mut commands: Commands,
        settings: Res<Settings>,
        mut active_cities: Query<(Entity, &mut Visibility), With<ActiveCity>>,
    ) {
        let (city_entity, mut visibility) = active_cities.single_mut();
        visibility.is_visible = true;
        commands.entity(city_entity).with_children(|parent| {
            parent.spawn_bundle(OrbitCameraBundle::new(settings.video.camera_render_graph()));
        });
    }

    fn exit_system(
        mut commands: Commands,
        mut active_cities: Query<(Entity, &mut Visibility), With<ActiveCity>>,
        cameras: Query<Entity, With<OrbitOrigin>>,
    ) {
        let (city_entity, mut visibility) = active_cities.single_mut();
        visibility.is_visible = false;
        commands.entity(city_entity).remove::<ActiveCity>();
        commands.entity(cameras.single()).despawn();
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
                .insert_bundle(TransformBundle::from_transform(transform))
                .insert_bundle(VisibilityBundle {
                    visibility: Visibility { is_visible: false },
                    ..Default::default()
                });
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

#[derive(Component)]
pub(crate) struct ActiveCity;

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default)]
struct PlacedCities(usize);
