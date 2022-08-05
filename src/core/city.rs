use bevy::prelude::*;

use super::game_world::InGameOnly;

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    in_mage_only: InGameOnly,
}

#[derive(Component, Default)]
pub(crate) struct City;
