use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use clap::{Args, Parser, Subcommand};

use super::{
    actor::ActiveActor,
    city::{ActiveCity, City},
    error::{self, ErrorReport},
    family::FamilySync,
    game_state::GameState,
    game_world::{GameLoad, WorldName, WorldState},
    network::{ConnectionSettings, ServerSettings},
};

/// Logic for command line interface.
///
/// This plugin expects [`Cli`] to be initialized early.
/// We do this to avoid creating a window for commands like `--help` or `--version`.
pub(super) struct CliPlugin;

impl Plugin for CliPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_startup_system(Self::subcommand_system.pipe(error::report))
            .add_system(
                Self::quick_loading_system
                    .pipe(error::report)
                    .in_set(OnUpdate(WorldState::InWorld))
                    .run_if(first_world_load),
            );
    }
}

impl CliPlugin {
    fn subcommand_system(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        cli: Res<Cli>,
        network_channels: Res<NetworkChannels>,
    ) -> Result<()> {
        if let Some(subcommand) = &cli.subcommand {
            match subcommand {
                GameCommand::Play(world_load) => {
                    load_events.send_default();
                    commands.insert_resource(WorldName(world_load.world_name.clone()));
                }
                GameCommand::Host {
                    world_load,
                    server_settings,
                } => {
                    let (server, transport) = server_settings
                        .create_server(
                            network_channels.server_channels(),
                            network_channels.client_channels(),
                        )
                        .context("unable to create server")?;
                    commands.insert_resource(server);
                    commands.insert_resource(transport);
                    commands.insert_resource(server_settings.clone());

                    commands.insert_resource(WorldName(world_load.world_name.clone()));
                    load_events.send_default();
                }
                GameCommand::Join(connection_settings) => {
                    let (client, transport) = connection_settings
                        .create_client(
                            network_channels.server_channels(),
                            network_channels.client_channels(),
                        )
                        .context("unable to create client")?;
                    commands.insert_resource(client);
                    commands.insert_resource(transport);
                    commands.insert_resource(connection_settings.clone());
                }
            }
        }

        Ok(())
    }

    fn quick_loading_system(
        mut commands: Commands,
        mut game_state: ResMut<NextState<GameState>>,
        cli: Res<Cli>,
        cities: Query<(Entity, &Name), With<City>>,
        families: Query<(Entity, &Name)>,
        actors: Query<(Entity, &FamilySync)>,
    ) -> Result<()> {
        if let Some(quick_load) = cli.get_quick_load() {
            match quick_load {
                QuickLoad::City { name } => {
                    let (entity, _) = cities
                        .iter()
                        .find(|(_, city_name)| city_name.as_str() == name)
                        .with_context(|| format!("unable to find city named {name}"))?;

                    commands.entity(entity).insert(ActiveCity);
                    game_state.set(GameState::City);
                }
                QuickLoad::Family { name } => {
                    let (family_entity, _) = families
                        .iter()
                        .find(|(_, family_name)| family_name.as_str() == name)
                        .with_context(|| format!("unable to find family named {name}"))?;

                    // Search using `FamilySync` because `Actors` component inserted to family on update.
                    let (actor_entity, _) = actors
                        .iter()
                        .find(|(_, family_sync)| family_sync.0 == family_entity)
                        .expect("family should contain at least one actor");
                    commands.entity(actor_entity).insert(ActiveActor);
                    game_state.set(GameState::Family);
                }
            }
        }

        Ok(())
    }
}

/// Returns `true` for the first full world load (including first update tick to apply components like [`FamilyActors`])
fn first_world_load(
    mut was_loaded: Local<bool>,
    error_events: EventReader<ErrorReport>,
    world_state: Res<State<WorldState>>,
) -> bool {
    if *was_loaded {
        return false;
    }

    // Mark as loaded when an error was occurred.
    if !error_events.is_empty() {
        *was_loaded = true;
        return false;
    }

    if world_state.0 == WorldState::NoWorld {
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
