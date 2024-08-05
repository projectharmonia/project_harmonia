use std::net::{IpAddr, Ipv4Addr};

use anyhow::{Context, Result};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use bevy_replicon_renet::{
    renet::{ConnectionConfig, RenetClient, RenetServer},
    RenetChannelsExt,
};
use clap::{Args, Parser, Subcommand};

use project_harmonia_base::{
    game_world::{
        actor::SelectedActor,
        city::{ActiveCity, City},
        family::FamilyMembers,
        GameLoad, WorldName, WorldState,
    },
    message::error_message,
    network::{self, DEFAULT_PORT},
};

/// Logic for command line interface.
///
/// This plugin expects [`Cli`] to be initialized early.
/// We do this to avoid creating a window for commands like `--help` or `--version`.
pub(super) struct CliPlugin;

impl Plugin for CliPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Startup, Self::apply_subcommand.pipe(error_message))
            .add_systems(
                Update,
                Self::quick_load.pipe(error_message).run_if(
                    in_state(WorldState::World)
                        // HACK: wait for family members initialiaztion.
                        // They initalized in `PreUpdate`, but state transitions happens later.
                        // Can be removed after switching to hooks.
                        .and_then(any_with_component::<FamilyMembers>)
                        .and_then(run_once()),
                ),
            );
    }
}

impl CliPlugin {
    fn apply_subcommand(
        mut commands: Commands,
        mut load_events: EventWriter<GameLoad>,
        cli: Res<Cli>,
        network_channels: Res<RepliconChannels>,
    ) -> Result<()> {
        if let Some(subcommand) = &cli.subcommand {
            match subcommand {
                GameCommand::Play(world_load) => {
                    load_events.send_default();
                    commands.insert_resource(WorldName(world_load.world_name.clone()));
                }
                GameCommand::Host { world_load, port } => {
                    let server = RenetServer::new(ConnectionConfig {
                        server_channels_config: network_channels.get_server_configs(),
                        client_channels_config: network_channels.get_client_configs(),
                        ..Default::default()
                    });
                    let transport =
                        network::create_server(*port).context("unable to create server")?;

                    commands.insert_resource(server);
                    commands.insert_resource(transport);
                    commands.insert_resource(WorldName(world_load.world_name.clone()));

                    load_events.send_default();
                }
                GameCommand::Join { ip, port } => {
                    let client = RenetClient::new(ConnectionConfig {
                        server_channels_config: network_channels.get_server_configs(),
                        client_channels_config: network_channels.get_client_configs(),
                        ..Default::default()
                    });
                    let transport =
                        network::create_client(*ip, *port).context("unable to create client")?;

                    commands.insert_resource(client);
                    commands.insert_resource(transport);
                }
            }
        }

        Ok(())
    }

    fn quick_load(
        mut commands: Commands,
        mut world_state: ResMut<NextState<WorldState>>,
        cli: Res<Cli>,
        cities: Query<(Entity, &Name), With<City>>,
        families: Query<(&Name, &FamilyMembers)>,
    ) -> Result<()> {
        if let Some(quick_load) = cli.quick_load() {
            match quick_load {
                QuickLoad::City { name } => {
                    let (entity, _) = cities
                        .iter()
                        .find(|(_, city_name)| city_name.as_str() == name)
                        .with_context(|| format!("unable to find city named {name}"))?;

                    commands.entity(entity).insert(ActiveCity);
                    world_state.set(WorldState::City);
                }
                QuickLoad::Family { name } => {
                    let (_, members) = families
                        .iter()
                        .find(|(family_name, _)| family_name.as_str() == name)
                        .with_context(|| format!("unable to find family named {name}"))?;

                    let entity = *members
                        .first()
                        .expect("family should contain at least one actor");
                    commands.entity(entity).insert(SelectedActor);
                    world_state.set(WorldState::Family);
                }
            }
        }

        Ok(())
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
    fn quick_load(&self) -> Option<&QuickLoad> {
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

        /// Port to use.
        #[clap(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
    Join {
        /// Server IP address.
        #[clap(short, long, default_value_t = Ipv4Addr::LOCALHOST.into())]
        ip: IpAddr,

        /// Server port.
        #[clap(short, long, default_value_t = DEFAULT_PORT)]
        port: u16,
    },
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
