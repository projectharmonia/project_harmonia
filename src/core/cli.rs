use clap::{Parser, Subcommand};

#[derive(Parser, Clone)]
#[clap(author, version, about)]
pub(crate) struct Cli {
    /// Game command to run.
    #[clap(subcommand)]
    pub(crate) subcommand: Option<GameCommand>,
}

impl Cli {
    /// Returns city to load if was specified from any subcommand.
    pub(crate) fn city(&self) -> Option<&String> {
        match &self.subcommand {
            Some(GameCommand::Play {
                world_name: _,
                city,
            }) => city.as_ref(),
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
        city: Option<String>,
    },
}
