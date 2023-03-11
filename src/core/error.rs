use anyhow::{Error, Result};
use bevy::prelude::*;

/// System adapter that logs errors and creates [`LastError`] resource.
pub(crate) fn report(In(result): In<Result<()>>, mut commands: Commands) {
    if let Err(error) = result {
        if cfg!(test) {
            eprintln!("{error}");
        } else {
            error!("{error}");
            commands.insert_resource(LastError(error));
        }
    }
}

/// Contains last error that was reported using [`report`] adapter.
#[derive(Resource)]
pub(crate) struct LastError(pub(crate) Error);
