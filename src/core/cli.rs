use anyhow::{Context, Result};
use bevy::prelude::*;
use clap::{Args, Parser, Subcommand};
use iyes_loopless::prelude::*;

use super::{
    city::City,
    error_message::{self, ErrorMessage},
    game_state::GameState,
    game_world::{GameLoad, GameWorld},
    network::{
        client::ConnectionSettings, network_event::NetworkEventCounter, server::ServerSettings,
    },
};

/// Logic for command line interface.
///
/// This plugin expects [`Cli`] to be initialized early.
/// We do this to avoid creating a window for commands like `--help` or `--version`.
pub(super) struct CliPlugin;

impl Plugin for CliPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(Self::subcommand_system.chain(error_message::err_message_system))
            .add_system(
                Self::city_loading_system
                    .chain(error_message::err_message_system)
                    .run_if(first_world_load),
            );
    }
}

impl CliPlugin {
    fn subcommand_system(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        cli: Res<Cli>,
        event_counter: Res<NetworkEventCounter>,
    ) -> Result<()> {
        if let Some(subcommand) = &cli.subcommand {
            match subcommand {
                GameCommand::Play(quick_load) => {
                    commands.insert_resource(GameWorld::new(quick_load.world_name.clone()));
                    load_events.send_default();
                }
                GameCommand::Host {
                    quick_load,
                    server_settings,
                } => {
                    let server = server_settings
                        .create_server(*event_counter)
                        .context("unable to create server")?;
                    commands.insert_resource(server);
                    commands.insert_resource(server_settings.clone());

                    commands.insert_resource(GameWorld::new(quick_load.world_name.clone()));
                    load_events.send_default();
                }
                GameCommand::Join(connection_settings) => {
                    let client = connection_settings
                        .create_client(*event_counter)
                        .context("unable to create client")?;
                    commands.insert_resource(client);
                    commands.insert_resource(connection_settings.clone());
                }
            }
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

fn first_world_load(
    mut was_loaded: Local<bool>,
    error_message: Option<Res<ErrorMessage>>,
    added_scenes: Query<(), Added<City>>,
) -> bool {
    if *was_loaded {
        return false;
    }

    // Mark as loaded when an error was occured.
    if error_message.is_some() {
        *was_loaded = true;
        return false;
    }

    if added_scenes.is_empty() {
        false
    } else {
        *was_loaded = true;
        true
    }
}

#[derive(Parser, Clone)]
#[command(author, version, about)]
pub(crate) struct Cli {
    /// Game command to run.
    #[command(subcommand)]
    subcommand: Option<GameCommand>,
}

impl Cli {
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
        Self::parse()
    }
}

#[derive(Subcommand, Clone)]
pub(crate) enum GameCommand {
    Play(QuickLoad),
    Host {
        #[command(flatten)]
        quick_load: QuickLoad,

        #[command(flatten)]
        server_settings: ServerSettings,
    },
    Join(ConnectionSettings),
}

/// Arguments for quick load.
#[derive(Args, Clone)]
pub(crate) struct QuickLoad {
    /// World name to load.
    #[arg(short, long)]
    world_name: String,

    /// City name to load.
    #[arg(short, long)]
    city_name: Option<String>,
}
