use anyhow::{Context, Result};
use bevy::prelude::*;
use clap::{Args, Parser, Subcommand};
use iyes_loopless::prelude::*;

use super::{
    city::{ActiveCity, City},
    error_message::{self, ErrorMessage},
    family::{Budget, FamilySelect},
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
                Self::quick_loading_system
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
                GameCommand::Play(world_load) => {
                    commands.insert_resource(GameWorld::new(world_load.world_name.clone()));
                    load_events.send_default();
                }
                GameCommand::Host {
                    world_load,
                    server_settings,
                } => {
                    let server = server_settings
                        .create_server(*event_counter)
                        .context("unable to create server")?;
                    commands.insert_resource(server);
                    commands.insert_resource(server_settings.clone());

                    commands.insert_resource(GameWorld::new(world_load.world_name.clone()));
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

    fn quick_loading_system(
        mut commands: Commands,
        mut select_events: EventWriter<FamilySelect>,
        cli: Res<Cli>,
        cities: Query<(Entity, &Name), With<City>>,
        families: Query<(Entity, &Name), With<Budget>>,
    ) -> Result<()> {
        if let Some(quick_load) = cli.get_quick_load() {
            match quick_load {
                QuickLoad::City { name } => {
                    let city_entity = cities
                        .iter()
                        .find(|(_, city_name)| city_name.as_str() == name)
                        .map(|(city, _)| city)
                        .with_context(|| format!("unable to find city named {name}"))?;

                    commands.entity(city_entity).insert(ActiveCity);
                    commands.insert_resource(NextState(GameState::City));
                }
                QuickLoad::Family { name } => {
                    let family_entity = families
                        .iter()
                        .find(|(_, family_name)| family_name.as_str() == name)
                        .map(|(family, _)| family)
                        .with_context(|| format!("unable to find family named {name}"))?;

                    select_events.send(FamilySelect(family_entity));
                }
            }
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
    fn get_quick_load(&self) -> Option<&QuickLoad> {
        match &self.subcommand {
            Some(GameCommand::Play(world_load)) => world_load.quick_load.as_ref(),
            Some(GameCommand::Host { world_load, .. }) => world_load.quick_load.as_ref(),
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
enum GameCommand {
    Play(WorldLoad),
    Host {
        #[command(flatten)]
        world_load: WorldLoad,

        #[command(flatten)]
        server_settings: ServerSettings,
    },
    Join(ConnectionSettings),
}

/// Arguments for quick load.
#[derive(Args, Clone)]
struct WorldLoad {
    /// World name to load.
    #[arg(short, long)]
    world_name: String,

    /// City name to load.
    #[command(subcommand)]
    quick_load: Option<QuickLoad>,
}

#[derive(Subcommand, Clone)]
enum QuickLoad {
    City { name: String },
    Family { name: String },
}
