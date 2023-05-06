use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

use crate::core::{
    cursor_hover::CursorHover,
    family::{ActorFamily, FamilyMode},
    game_state::GameState,
    ground::Ground,
    task::{ReflectTask, Task, TaskList},
};

use super::{LotFamily, LotVertices};

pub(super) struct BuyLotPlugin;

impl Plugin for BuyLotPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<BuyLot>()
            .register_component_as::<dyn Task, BuyLot>()
            .add_system(
                Self::tasks_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_system(Self::buying_system.in_set(ServerSet::Authority));
    }
}

impl BuyLotPlugin {
    fn tasks_system(
        mut grounds: Query<(&CursorHover, &mut TaskList), (With<Ground>, Added<TaskList>)>,
        lots: Query<(Entity, &LotVertices), Without<LotFamily>>,
    ) {
        if let Ok((hover, mut task_list)) = grounds.get_single_mut() {
            let position = hover.xz();
            if let Some((lot_entity, _)) = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(position))
            {
                task_list.push(Box::new(BuyLot(lot_entity)));
            }
        }
    }

    fn buying_system(
        mut commands: Commands,
        lots: Query<(), Without<LotFamily>>,
        actors: Query<(Entity, &ActorFamily, &BuyLot), Added<BuyLot>>,
    ) {
        for (entity, family, buy) in &actors {
            if lots.get(buy.0).is_ok() {
                commands.entity(buy.0).insert(LotFamily(family.0));
            } else {
                error!("{buy:?} from actor {entity:?} points to not a lot");
            }
            commands.entity(entity).remove::<BuyLot>();
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component, Task)]
pub(crate) struct BuyLot(Entity);

impl Task for BuyLot {
    fn name(&self) -> &str {
        "Buy lot"
    }
}

impl FromWorld for BuyLot {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

impl MapEntities for BuyLot {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}
