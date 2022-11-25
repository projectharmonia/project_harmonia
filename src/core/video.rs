use bevy::prelude::*;
use bevy_hikari::prelude::*;
use iyes_loopless::prelude::IntoConditionalSystem;

use super::{
    game_world::GameWorld,
    settings::{Settings, SettingsApply},
};

pub(super) struct VideoPlugin;

impl Plugin for VideoPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(Self::init_cameras.run_if_resource_exists::<GameWorld>())
            .add_system(Self::toggle_global_illumination_system.run_on_event::<SettingsApply>());
    }
}

impl VideoPlugin {
    fn toggle_global_illumination_system(
        mut commands: Commands,
        settings: Res<Settings>,
        mut cameras: Query<Entity, With<Camera3d>>,
    ) {
        for entity in &mut cameras {
            if settings.video.global_illumination {
                commands.entity(entity).insert(HikariSettings::default());
            } else {
                commands.entity(entity).remove::<HikariSettings>();
            }
        }
    }

    fn init_cameras(
        mut commands: Commands,
        settings: Res<Settings>,
        mut new_cameras: Query<Entity, Added<Camera3d>>,
    ) {
        for entity in &mut new_cameras {
            if settings.video.global_illumination {
                commands.entity(entity).insert(HikariSettings::default());
            } else {
                commands.entity(entity).remove::<HikariSettings>();
            }
        }
    }
}
