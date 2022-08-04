use anyhow::Result;
use bevy::prelude::*;

/// A system to enable early return with error logging as errors.
pub(super) fn log_err_system(In(result): In<Result<()>>) {
    if let Err(error) = result {
        error!("{error:#}")
    }
}
