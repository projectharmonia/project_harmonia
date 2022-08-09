use bevy::prelude::*;

use super::game_world::GameEntity;

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
    game_entity: GameEntity,
}

#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(crate) struct Family(Vec<Entity>);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Budget(u32);
