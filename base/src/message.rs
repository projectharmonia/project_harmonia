use anyhow::Result;
use bevy::prelude::*;

pub(super) struct ErrorReportPlugin;

impl Plugin for ErrorReportPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<Message>();
    }
}

/// System adapter that logs errors and sends [`Message`] event.
pub fn error_message(In(result): In<Result<()>>, mut error_events: EventWriter<Message>) {
    if let Err(error) = result {
        error!("{error}");
        error_events.send(Message(format!("Error: {error:#}")));
    }
}

/// Contains error that was reported using [`error_message`] adapter.
#[derive(Event)]
pub struct Message(pub String);
