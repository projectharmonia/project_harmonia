use anyhow::{Context, Result};
use bevy::prelude::*;
use clap::{Args, Parser, Subcommand};
use iyes_loopless::prelude::*;

use super::{
    city::{ActiveCity, City},
    doll::ActiveDoll,
    error::{self, LastError},
    family::Dolls,
    game_state::GameState,
    game_world::GameLoad,
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
        app.add_startup_system(Self::subcommand_system.pipe(error::report))
            .add_system_to_stage(
                CoreStage::PostUpdate, // To run after `Dolls` component insertion.
                Self::quick_loading_system
                    .pipe(error::report)
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
                    load_events.send(GameLoad(world_load.world_name.clone()));
                    commands.insert_resource(NextState(GameState::World));
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

                    load_events.send(GameLoad(world_load.world_name.clone()));
                    commands.insert_resource(NextState(GameState::World));
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
        cli: Res<Cli>,
        cities: Query<(Entity, &Name), With<City>>,
        families: Query<(&Dolls, &Name)>,
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
                    let dolls = families
                        .iter()
                        .find(|(.., family_name)| family_name.as_str() == name)
                        .map(|(dolls, _)| dolls)
                        .with_context(|| format!("unable to find family named {name}"))?;

                    let doll_entity = *dolls
                        .first()
                        .expect("family should contain at least one doll");
                    commands.entity(doll_entity).insert(ActiveDoll);
                    commands.insert_resource(NextState(GameState::Family))
                }
            }
        }

        Ok(())
    }
}

fn first_world_load(
    mut was_loaded: Local<bool>,
    error_message: Option<Res<LastError>>,
    added_scenes: Query<(), Added<City>>,
) -> bool {
    if *was_loaded {
        return false;
    }

    // Mark as loaded when an error was occurred.
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

#[derive(Parser, Clone, Resource)]
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
