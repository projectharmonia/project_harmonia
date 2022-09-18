use anyhow::{Error, Result};
use bevy::prelude::*;

/// A system to enable early return with error message reporting.
#[cfg_attr(coverage, no_coverage)]
pub(crate) fn err_message_system(
    In(result): In<Result<()>>,
    #[cfg(not(test))] mut commands: Commands,
) {
    if let Err(error) = result {
        #[cfg(test)]
        eprintln!("{error}");
        #[cfg(not(test))]
        commands.insert_resource(ErrorMessage(error));
    }
}

pub(crate) struct ErrorMessage(pub(crate) Error);
