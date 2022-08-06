use bevy::prelude::*;

use super::game_world::InGameOnly;

pub(super) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Family>().register_type::<Budget>();
    }
}

#[derive(Bundle, Default)]
pub(crate) struct FamilyBundle {
    name: Name,
    family: Family,
    budget: Budget,
    in_game_only: InGameOnly,
}

#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(crate) struct Family(Vec<Entity>);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Budget(u32);
