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

    #[bundle]
    spatial: SpatialBundle,
}

impl CityBundle {
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            city: City,
            in_mage_only: InGameOnly,
            spatial: Default::default(),
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct City;
