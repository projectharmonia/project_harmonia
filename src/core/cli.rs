use anyhow::{Context, Result};
use bevy::prelude::*;
use clap::{Parser, Subcommand};
use iyes_loopless::prelude::*;

use super::{
    city::City,
    errors,
    game_state::GameState,
    game_world::{GameLoaded, GameWorld},
};

/// Logic for command line interface.
///
/// This plugin expects [`Cli`] to be initialized early.
/// We do this to avoid creating a window for commands like `--help` or `--version`.
pub(super) struct CliPlugin;

impl Plugin for CliPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(Self::world_loading_system.chain(errors::log_err_system))
            .add_system(
                Self::city_loading_system
                    .chain(errors::log_err_system)
                    .run_if(is_world_loaded_once),
            );
    }
}

impl CliPlugin {
    fn world_loading_system(
        mut commands: Commands,
        mut load_events: ResMut<Events<GameLoaded>>,
        cli: Res<Cli>,
    ) -> Result<()> {
        if let Some(world_name) = cli.world_name() {
            commands.insert_resource(GameWorld::new(world_name.clone()));
            load_events.send_default();
            // Should be called to avoid other systems reacting on the event twice
            // See https://github.com/IyesGames/iyes_loopless/issues/31
            load_events.update();
        }

        Ok(())
    }

    fn city_loading_system(
        mut commands: Commands,
        cli: Res<Cli>,
        cities: Query<(Entity, &Name), With<City>>,
    ) -> Result<()> {
        if let Some(city_name) = cli.city_name() {
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

#[derive(Parser, Clone)]
#[clap(author, version, about)]
pub(crate) struct Cli {
    /// Game command to run.
    #[clap(subcommand)]
    pub(crate) subcommand: Option<GameCommand>,
}

impl Cli {
    /// Returns city to load if was specified from any subcommand.
    pub(crate) fn city_name(&self) -> Option<&String> {
        match &self.subcommand {
            Some(GameCommand::Play {
                world_name: _,
                city_name,
            }) => city_name.as_ref(),
            None => None,
        }
    }

    /// Returns world to load if was specified from any subcommand.
    pub(crate) fn world_name(&self) -> Option<&String> {
        match &self.subcommand {
            Some(GameCommand::Play { world_name, .. }) => Some(world_name),
            None => None,
        }
    }
}

impl Default for Cli {
    fn default() -> Self {
        #[cfg(test)]
        return Self { subcommand: None };
        #[cfg(not(test))]
        return Self::parse();
    }
}

#[derive(Subcommand, Clone)]
pub(crate) enum GameCommand {
    Play {
        /// World name to load.
        #[clap(short, long)]
        world_name: String,

        /// City name to load.
        #[clap(short, long)]
        city_name: Option<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::city::CityBundle;

    #[test]
    fn loading_world() {
        const WORLD_NAME: &str = "World from CLI";
        let mut app = App::new();
        app.add_event::<GameLoaded>()
            .insert_resource(Cli {
                subcommand: Some(GameCommand::Play {
                    world_name: WORLD_NAME.to_string(),
                    city_name: None,
                }),
            })
            .add_plugin(CliPlugin);

        app.update();

        assert_eq!(app.world.resource::<Events<GameLoaded>>().len(), 1);
        assert_eq!(app.world.resource::<GameWorld>().world_name, WORLD_NAME);
    }

    #[test]
    fn loading_city() {
        let mut app = App::new();
        app.add_event::<GameLoaded>().add_plugin(CliPlugin);

        const CITY_NAME: &str = "City from CLI";
        let city_entity = app
            .world
            .spawn()
            .insert_bundle(CityBundle::new(CITY_NAME.into()))
            .id();

        app.insert_resource(Cli {
            subcommand: Some(GameCommand::Play {
                world_name: String::new(),
                city_name: Some(CITY_NAME.to_string()),
            }),
        });

        app.update();

        assert!(app.world.entity(city_entity).contains::<Visibility>());
        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::City,
        );
    }
}
