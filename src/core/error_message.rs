use anyhow::{Error, Result};
use bevy::prelude::*;

/// A system to enable early return with error message reporting.
pub(crate) fn err_message_system(In(result): In<Result<()>>, mut commands: Commands) {
    if let Err(error) = result {
        if cfg!(test) {
            eprintln!("{error}");
        } else {
            commands.insert_resource(ErrorMessage(error));
        }
    }
}

#[derive(Resource)]
pub(crate) struct ErrorMessage(pub(crate) Error);
