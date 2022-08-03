use bevy::prelude::*;

use super::game_state::InGameOnly;

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    in_mage_only: InGameOnly,
}

#[derive(Component, Default)]
pub(crate) struct City;
