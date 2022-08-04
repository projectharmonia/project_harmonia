use anyhow::Result;
use bevy::prelude::*;
use tap::TapFallible;

/// A system to enable early return with error logging as errors.
pub(super) fn log_err_system(In(result): In<Result<()>>) {
    result.tap_err(|e| error!("{e:#}")).ok();
}
