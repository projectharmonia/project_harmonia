use bevy::prelude::*;

use super::game_world::InGameOnly;

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<City>();
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    in_mage_only: InGameOnly,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct City;
