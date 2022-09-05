use anyhow::{Context, Result};
use bevy::prelude::*;
use clap::{Args, Parser, Subcommand};
use iyes_loopless::prelude::*;

use super::{
    city::City,
    errors,
    game_state::GameState,
    game_world::{GameLoaded, GameWorld},
    network::{client::ConnectionSettings, server::ServerSettings},
};

/// Logic for command line interface.
///
/// This plugin expects [`Cli`] to be initialized early.
/// We do this to avoid creating a window for commands like `--help` or `--version`.
pub(super) struct CliPlugin;

impl Plugin for CliPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(Self::world_loading_system.chain(errors::log_err_system))
            .add_startup_system(Self::host_system.chain(errors::log_err_system))
            .add_startup_system(Self::join_system.chain(errors::log_err_system))
            .add_system(
                Self::city_loading_system
                    .chain(errors::log_err_system)
                    .run_if(is_world_loaded_once),
            );
    }
}

impl CliPlugin {
    fn host_system(mut commands: Commands, cli: Res<Cli>) -> Result<()> {
        if let Some(GameCommand::Host {
            quick_load: _,
            server_settings,
        }) = &cli.subcommand
        {
            let server = server_settings
                .create_server()
                .context("unable to create server")?;
            commands.insert_resource(server);
            commands.insert_resource(server_settings.clone());
        }

        Ok(())
    }

    fn join_system(mut commands: Commands, cli: Res<Cli>) -> Result<()> {
        if let Some(GameCommand::Join(connection_settings)) = &cli.subcommand {
            let client = connection_settings
                .create_client()
                .context("unable to create client")?;
            commands.insert_resource(client);
            commands.insert_resource(connection_settings.clone());
        }

        Ok(())
    }

    fn world_loading_system(
        mut commands: Commands,
        mut load_events: ResMut<Events<GameLoaded>>,
        cli: Res<Cli>,
    ) -> Result<()> {
        if let Some(QuickLoad { world_name, .. }) = cli.get_quick_load() {
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
        if let Some(city_name) = cli
            .get_quick_load()
            .and_then(|quick_load| quick_load.city_name.as_ref())
        {
            let city_entity = cities
                .iter()
                .find(|(_, name)| name.as_str() == city_name)
                .map(|(city, _)| city)
                .with_context(|| format!("unable to find city named {city_name}"))?;

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
    /// Creates an instance with `GameMode::Play` variant.
    #[cfg(test)]
    fn with_play(world_name: String, city_name: Option<String>) -> Self {
        Self {
            subcommand: Some(GameCommand::Play(QuickLoad {
                world_name,
                city_name,
            })),
        }
    }

    /// Creates an instance with `GameMode::Host` variant.
    #[cfg(test)]
    fn with_host(
        world_name: String,
        city_name: Option<String>,
        server_settings: ServerSettings,
    ) -> Self {
        Self {
            subcommand: Some(GameCommand::Host {
                quick_load: QuickLoad {
                    world_name,
                    city_name,
                },
                server_settings,
            }),
        }
    }

    /// Creates an instance with `GameMode::Join` variant.
    #[cfg(test)]
    fn with_join(connection_settings: ConnectionSettings) -> Self {
        Self {
            subcommand: Some(GameCommand::Join(connection_settings)),
        }
    }

    /// Returns arguments for quick load if was specified from any subcommand.
    pub(crate) fn get_quick_load(&self) -> Option<&QuickLoad> {
        match &self.subcommand {
            Some(GameCommand::Play(quick_load)) => Some(quick_load),
            Some(GameCommand::Host { quick_load, .. }) => Some(quick_load),
            _ => None,
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
    Play(QuickLoad),
    Host {
        #[clap(flatten)]
        quick_load: QuickLoad,

        #[clap(flatten)]
        server_settings: ServerSettings,
    },
    Join(ConnectionSettings),
}

/// Arguments for quick load.
#[derive(Args, Clone)]
pub(crate) struct QuickLoad {
    /// World name to load.
    #[clap(short, long)]
    world_name: String,

    /// City name to load.
    #[clap(short, long)]
    city_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use bevy_renet::renet::{RenetClient, RenetServer};

    use super::*;
    use crate::core::city::CityBundle;

    const WORLD_NAME: &str = "World from CLI";

    #[test]
    fn loading_world() {
        let mut app = App::new();
        app.add_event::<GameLoaded>()
            .insert_resource(Cli::with_play(WORLD_NAME.to_string(), None))
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

        app.insert_resource(Cli::with_play(String::new(), Some(CITY_NAME.to_string())));

        app.update();

        assert!(app.world.entity(city_entity).contains::<Visibility>());
        assert_eq!(
            app.world.resource::<NextState<GameState>>().0,
            GameState::City,
        );
    }

    #[test]
    fn hosting() {
        let mut app = App::new();
        app.add_event::<GameLoaded>().add_plugin(CliPlugin);
        let server_settings = ServerSettings {
            port: 0,
            ..Default::default()
        };
        app.world.insert_resource(Cli::with_host(
            WORLD_NAME.to_string(),
            None,
            server_settings.clone(),
        ));

        app.update();

        assert_eq!(*app.world.resource::<ServerSettings>(), server_settings);
        assert!(app.world.get_resource::<RenetServer>().is_some());
    }

    #[test]
    fn joining() {
        let mut app = App::new();
        app.add_event::<GameLoaded>().add_plugin(CliPlugin);
        let connection_settings = ConnectionSettings {
            port: 0,
            ..Default::default()
        };
        app.world
            .insert_resource(Cli::with_join(connection_settings.clone()));

        app.update();

        assert_eq!(
            *app.world.resource::<ConnectionSettings>(),
            connection_settings,
        );
        assert!(app.world.get_resource::<RenetClient>().is_some());
    }
}
