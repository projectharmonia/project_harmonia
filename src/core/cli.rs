use clap::{Parser, Subcommand};

#[derive(Parser, Clone)]
#[clap(author, version, about)]
pub(crate) struct Cli {
    /// Game command to run.
    #[clap(subcommand)]
    pub(crate) subcommand: Option<GameCommand>,
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
    },
}
