use bevy::prelude::*;

pub(super) struct RootPlugin;

impl Plugin for RootPlugin {
    fn build(&self, app: &mut App) {
        // Bevy requires that there should be only a single root.
        //
        // We spawn a root entity that takes the whole space and attach all UI to it.
        //
        // Should be done even before `Startup` since state transitions run earlier
        // and main menu may try to access it since it's the default state.
        debug!("spawning root UI node");
        app.world_mut().spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}
