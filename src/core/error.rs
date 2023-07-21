use anyhow::{Error, Result};
use bevy::prelude::*;

pub(super) struct ErrorPlugin;

impl Plugin for ErrorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ErrorReport>();
    }
}

/// System adapter that logs errors and creates [`LastError`] resource.
pub(crate) fn report(
    In(result): In<Result<()>>,
    #[cfg(not(test))] mut error_events: EventWriter<ErrorReport>,
) {
    if let Err(error) = result {
        #[cfg(test)]
        eprintln!("{error}");
        #[cfg(not(test))]
        {
            error!("{error}");
            error_events.send(ErrorReport(error));
        }
    }
}

/// Contains error that was reported using [`report`] adapter.
#[derive(Event)]
pub(crate) struct ErrorReport(pub(crate) Error);
