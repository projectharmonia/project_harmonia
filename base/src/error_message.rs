use anyhow::Result;
use bevy::prelude::*;

/// System adapter that logs errors and sends [`ErrorMessage`] event.
pub fn error_message(In(result): In<Result<()>>, mut commands: Commands) {
    if let Err(error) = result {
        error!("{error:#}");
        commands.trigger(ErrorMessage(format!("Error: {error:#}")));
    }
}

/// Contains error that was reported using [`error_message`] adapter.
#[derive(Event, Deref)]
pub struct ErrorMessage(String);

impl ErrorMessage {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}
