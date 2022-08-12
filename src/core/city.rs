use anyhow::{Context, Result};
use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::game_state::GameState;

use super::{
    cli::{Cli, GameCommand},
    errors::log_err_system,
    game_world::{GameEntity, GameWorld},
};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedCities>()
            .register_type::<City>()
            .add_system(
                Self::load_from_cli
                    .chain(log_err_system)
                    .run_if_resource_added::<Cli>(),
            )
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
        if let Some(GameCommand::Play {
            world_name: _,
            city: Some(load_city),
        }) = &cli.subcommand
        {
            let city = cities
                .iter()
                .find(|(_, name)| name.as_str() == load_city)
                .map(|(city, _)| city)
                .with_context(|| format!("Unable to find city named {load_city}"))?;

            commands
                .entity(city)
                .insert_bundle(VisibilityBundle::default());
            commands.insert_resource(NextState(GameState::City));
        }

        Ok(())
    }

    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn placement_system(
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, Added<City>>,
    ) {
        for city in &added_cities {
            let transform =
                Transform::from_translation(Vec3::X * Self::CITY_SIZE * placed_citites.0 as f32);
            commands
                .entity(city)
                .insert_bundle(TransformBundle::from_transform(transform));
            placed_citites.0 += 1;
        }
    }

    /// Resets [`PlacedCities`] counter to 0.
    fn reset_paced_cities_system(mut placed_citites: ResMut<PlacedCities>) {
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

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default)]
struct PlacedCities(usize);

#[cfg(test)]
mod tests {
    use std::any;

    use super::*;

    #[test]
    fn loading_from_cli() {
        let mut app = App::new();
        app.add_plugin(CityPlugin);

        const CITY_NAME: &str = "City from CLI";
        let city = app
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

        assert!(
            app.world.entity(city).contains::<Visibility>(),
            "{} component should be added to the selected city",
            any::type_name::<Visibility>()
        );
        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::City,
            "State should be changed to {}",
            GameState::City
        );
    }

    #[test]
    fn placing() {
        let mut app = App::new();
        app.init_resource::<GameWorld>().add_plugin(CityPlugin);

        app.update();

        for index in 0..2 {
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

    #[test]
    fn placed_citites_reset() {
        let mut app = App::new();
        app.init_resource::<GameWorld>().add_plugin(CityPlugin);

        app.world.resource_mut::<PlacedCities>().0 += 1;

        app.update();

        app.world.remove_resource::<GameWorld>();
        app.update();

        assert_eq!(
            app.world.resource::<PlacedCities>().0,
            0,
            "Number of placed cities should be resetted after removing {}",
            any::type_name::<GameWorld>(),
        );
    }
}
