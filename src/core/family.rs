use bevy::prelude::*;

use super::game_world::InGameOnly;

#[derive(Bundle, Default)]
pub(crate) struct FamilyBundle {
    name: Name,
    family: Family,
    budget: Budget,
    in_game_only: InGameOnly,
}

#[derive(Component, Default, Deref, DerefMut)]
pub(crate) struct Family(Vec<Entity>);

#[derive(Component, Default)]
pub(crate) struct Budget(u32);
