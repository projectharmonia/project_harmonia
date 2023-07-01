use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    cursor_hover::CursorHover,
    family::ActorFamily,
    ground::Ground,
    task::{AppTaskExt, Task, TaskList, TaskListSet, TaskState},
};

use super::{LotFamily, LotVertices};

pub(super) struct BuyLotPlugin;

impl Plugin for BuyLotPlugin {
    fn build(&self, app: &mut App) {
        app.register_task::<BuyLot>()
            .add_system(Self::list_system.in_set(TaskListSet))
            .add_system(Self::buying_system.in_set(ServerSet::Authority));
    }
}

impl BuyLotPlugin {
    fn list_system(
        mut list_events: EventWriter<TaskList>,
        mut grounds: Query<&CursorHover, With<Ground>>,
        lots: Query<(Entity, &LotVertices), Without<LotFamily>>,
    ) {
        if let Ok(hover) = grounds.get_single_mut() {
            let position = hover.xz();
            if let Some((lot_entity, _)) = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(position))
            {
                list_events.send(BuyLot(lot_entity).into());
            }
        }
    }

    fn buying_system(
        mut commands: Commands,
        lots: Query<(), Without<LotFamily>>,
        actors: Query<&ActorFamily>,
        tasks: Query<(Entity, &Parent, &BuyLot, &TaskState), Changed<TaskState>>,
    ) {
        for (entity, parent, buy, &state) in &tasks {
            if state == TaskState::Active {
                let family = actors
                    .get(**parent)
                    .expect("actors should have assigned family");
                if lots.get(buy.0).is_ok() {
                    commands.entity(buy.0).insert(LotFamily(family.0));
                } else {
                    error!("{buy:?} from actor {entity:?} points to not a lot");
                }
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
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
