use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{AvailableTasks, ListTasks, Task, TaskState};
use crate::game_world::{
    actor::Actor,
    city::{
        lot::{LotFamily, LotVertices},
        Ground,
    },
};

pub(super) struct BuyLotPlugin;

impl Plugin for BuyLotPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BuyLot>()
            .replicate::<BuyLot>()
            .observe(Self::add_to_list)
            .add_systems(Update, Self::buy.run_if(server_or_singleplayer));
    }
}

impl BuyLotPlugin {
    fn add_to_list(
        trigger: Trigger<ListTasks>,
        mut available_tasks: ResMut<AvailableTasks>,
        grounds: Query<(), With<Ground>>,
        lots: Query<(Entity, &LotVertices), Without<LotFamily>>,
    ) {
        if grounds.get(trigger.entity()).is_ok() {
            if let Some((lot_entity, _)) = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(trigger.event().xz()))
            {
                available_tasks.add(BuyLot(lot_entity));
            }
        }
    }

    fn buy(
        mut commands: Commands,
        lots: Query<(), Without<LotFamily>>,
        actors: Query<&Actor>,
        tasks: Query<(Entity, &Parent, &BuyLot, &TaskState), Changed<TaskState>>,
    ) {
        for (entity, parent, buy, &task_state) in &tasks {
            if task_state == TaskState::Active {
                let actor = actors
                    .get(**parent)
                    .expect("task should have assigned actors");
                if lots.get(buy.0).is_ok() {
                    commands
                        .entity(buy.0)
                        .insert(LotFamily(actor.family_entity));
                } else {
                    error!("`{buy:?}` from actor `{entity}` points to not a lot");
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
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.0 = entity_mapper.map_entity(self.0);
    }
}
