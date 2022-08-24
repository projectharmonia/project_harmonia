use anyhow::{Context, Result};
use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::game_state::GameState;

use super::{cli::Cli, errors::log_err_system, game_world::GameWorld};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedCities>()
            .register_type::<City>()
            .add_system(
                Self::load_from_cli
                    .chain(log_err_system)
                    .run_if(is_world_loaded_once),
            )
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>())
            .add_system(Self::placement_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::reset_paced_cities_system.run_if_resource_removed::<GameWorld>());
    }
}

impl CityPlugin {
    /// Size of one size of a city.
    const CITY_SIZE: f32 = 100.0;

    fn load_from_cli(
        mut commands: Commands,
        cli: Res<Cli>,
        cities: Query<(Entity, &Name), With<City>>,
    ) -> Result<()> {
        if let Some(city_name) = cli.city() {
            let city_entity = cities
                .iter()
                .find(|(_, name)| name.as_str() == city_name)
                .map(|(city, _)| city)
                .with_context(|| format!("Unable to find city named {city_name}"))?;

            commands
                .entity(city_entity)
                .insert_bundle(VisibilityBundle::default());
            commands.insert_resource(NextState(GameState::City));
        }

        Ok(())
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
    fn reset_paced_cities_system(mut placed_citites: ResMut<PlacedCities>) {
        placed_citites.0 = 0;
    }
}

fn is_world_loaded_once(mut was_called: Local<bool>, added_scenes: Query<(), Added<City>>) -> bool {
    if *was_called {
        return false;
    }

    if added_scenes.is_empty() {
        false
    } else {
        *was_called = true;
        true
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
}

impl CityBundle {
    pub(crate) fn new(name: Name) -> Self {
        Self { name, city: City }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cli::GameCommand;

    #[test]
    fn loading_from_cli() {
        let mut app = App::new();
        app.add_plugin(CityPlugin);

        const CITY_NAME: &str = "City from CLI";
        let city_entity = app
            .world
            .spawn()
            .insert_bundle(CityBundle::new(CITY_NAME.into()))
            .id();

        app.insert_resource(Cli {
            subcommand: Some(GameCommand::Play {
                world_name: String::new(),
                city: Some(CITY_NAME.to_string()),
            }),
        });

        app.update();

        assert!(app.world.entity(city_entity).contains::<Visibility>());
        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::City,
        );
    }

    #[test]
    fn cleanup() {
        let mut app = App::new();
        app.init_resource::<Cli>()
            .init_resource::<GameWorld>()
            .add_plugin(CityPlugin);

        let child_entity = app.world.spawn().id();
        let city_entity = app
            .world
            .spawn()
            .insert(City)
            .push_children(&[child_entity])
            .id();

        app.update();

        app.world.remove_resource::<GameWorld>();

        app.update();

        assert!(app.world.get_entity(city_entity).is_none());
        assert!(app.world.get_entity(child_entity).is_none());
    }

    #[test]
    fn placing() {
        let mut app = App::new();
        app.init_resource::<Cli>()
            .init_resource::<GameWorld>()
            .add_plugin(CityPlugin);

        app.update();

        for index in 0..2 {
            let city_entity = app.world.spawn().insert_bundle(CityBundle::default()).id();

            app.update();

            let transform = *app
                .world
                .get::<Transform>(city_entity)
                .unwrap_or_else(|| panic!("Added city {index} should be placed"));

            assert_eq!(
                transform,
                Transform::from_translation(Vec3::X * CityPlugin::CITY_SIZE * index as f32),
                "City {index} should be placed with offset",
            );
        }
    }

    #[test]
    fn placed_citites_reset() {
        let mut app = App::new();
        app.init_resource::<GameWorld>().add_plugin(CityPlugin);

        app.world.resource_mut::<PlacedCities>().0 += 1;

        app.update();

        app.world.remove_resource::<GameWorld>();
        app.update();

        assert_eq!(app.world.resource::<PlacedCities>().0, 0);
    }
}
