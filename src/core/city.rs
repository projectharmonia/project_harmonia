use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::game_world::{GameEntity, GameWorld};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<City>()
            .add_system(Self::placement_system.run_if_resource_exists::<GameWorld>());
    }
}

impl CityPlugin {
    /// Size of one size of a city.
    const CITY_SIZE: f32 = 100.0;

    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn placement_system(
        mut commands: Commands,
        added_cities: Query<Entity, Added<City>>,
        placed_cities: Query<(), (With<City>, With<Transform>)>,
    ) {
        if added_cities.is_empty() {
            return;
        }

        let mut placed_cities = placed_cities.iter().count();
        for city in &added_cities {
            let transform =
                Transform::from_translation(Vec3::X * Self::CITY_SIZE * placed_cities as f32);
            commands
                .entity(city)
                .insert_bundle(TransformBundle::from_transform(transform));
            placed_cities += 1;
        }
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    game_world: GameEntity,
}

impl CityBundle {
    #[cfg_attr(coverage, no_coverage)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placing() {
        let mut app = App::new();
        app.init_resource::<GameWorld>().add_plugin(CityPlugin);

        app.update();

        for index in 0..3 {
            let city = app.world.spawn().insert_bundle(CityBundle::default()).id();

            app.update();

            let transform = *app
                .world
                .get::<Transform>(city)
                .unwrap_or_else(|| panic!("Added city {index} should be placed"));

            assert_eq!(
                transform,
                Transform::from_translation(Vec3::X * CityPlugin::CITY_SIZE * index as f32),
                "City {index} should be placed with offset",
            );
        }
    }
}
