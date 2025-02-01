use bevy::prelude::*;

pub(super) struct RootPlugin;

impl Plugin for RootPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn);
    }
}

fn spawn(mut commands: Commands) {
    debug!("spawning root UI node");

    commands.spawn((
        Name::new("UI root"),
        PickingBehavior::IGNORE,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..Default::default()
        },
    ));
}
