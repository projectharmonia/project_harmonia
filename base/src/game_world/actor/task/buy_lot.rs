use bevy::{
    ecs::entity::{EntityMapper, MapEntities},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::game_world::{
    actor::{
        task::{Task, TaskList, TaskListSet, TaskState},
        Actor,
    },
    city::{
        lot::{LotFamily, LotVertices},
        Ground,
    },
    hover::Hovered,
};

pub(super) struct BuyLotPlugin;

impl Plugin for BuyLotPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<BuyLot>()
            .replicate::<BuyLot>()
            .add_systems(
                Update,
                (
                    Self::add_to_list.in_set(TaskListSet),
                    Self::buy.run_if(server_or_singleplayer),
                ),
            );
    }
}

impl BuyLotPlugin {
    fn add_to_list(
        mut list_events: EventWriter<TaskList>,
        mut grounds: Query<&Hovered, With<Ground>>,
        lots: Query<(Entity, &LotVertices), Without<LotFamily>>,
    ) {
        if let Ok(point) = grounds.get_single_mut().map(|point| point.xz()) {
            if let Some((lot_entity, _)) = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(point))
            {
                list_events.send(BuyLot(lot_entity).into());
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
