pub mod wall;

use bevy::prelude::*;
use strum::EnumIter;

use super::FamilyMode;
use wall::WallPlugin;

pub(super) struct BuildingPlugin;

impl Plugin for BuildingPlugin {
    fn build(&self, app: &mut App) {
        app.add_sub_state::<BuildingMode>()
            .enable_state_scoped_entities::<BuildingMode>()
            .add_plugins(WallPlugin);
    }
}

#[derive(Clone, Copy, Component, Debug, Default, EnumIter, Eq, Hash, PartialEq, SubStates)]
#[source(FamilyMode = FamilyMode::Building)]
pub enum BuildingMode {
    #[default]
    Objects,
    Walls,
}

impl BuildingMode {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "💺",
            Self::Walls => "🔰",
        }
    }
}
