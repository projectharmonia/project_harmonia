use bevy::{
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    math::Vec3Swizzles,
    prelude::*,
};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::{
    cursor_hover::CursorHover,
    family::{ActorFamily, FamilyMode},
    game_state::GameState,
    game_world::WorldState,
    ground::Ground,
    task::{ActiveTask, AppTaskExt, ListedTask, TaskGroups, TaskList},
};

use super::{LotFamily, LotVertices};

pub(super) struct BuyLotPlugin;

impl Plugin for BuyLotPlugin {
    fn build(&self, app: &mut App) {
        app.register_task::<BuyLot>()
            .add_system(
                Self::list_system
                    .in_set(OnUpdate(GameState::Family))
                    .in_set(OnUpdate(FamilyMode::Life)),
            )
            .add_system(Self::init_system.in_set(OnUpdate(WorldState::InWorld)))
            .add_system(Self::buying_system.in_set(ServerSet::Authority));
    }
}

impl BuyLotPlugin {
    fn list_system(
        mut commands: Commands,
        grounds: Query<(Entity, &CursorHover), (With<Ground>, Added<TaskList>)>,
        lots: Query<(Entity, &LotVertices), Without<LotFamily>>,
    ) {
        if let Ok((entity, hover)) = grounds.get_single() {
            let position = hover.xz();
            if let Some((lot_entity, _)) = lots
                .iter()
                .find(|(_, vertices)| vertices.contains_point(position))
            {
                commands.entity(entity).with_children(|parent| {
                    parent.spawn((ListedTask, BuyLot(lot_entity)));
                });
            }
        }
    }

    fn init_system(mut commands: Commands, tasks: Query<Entity, Added<BuyLot>>) {
        for entity in &tasks {
            commands
                .entity(entity)
                .insert((Name::new("Buy lot"), TaskGroups::default()));
        }
    }

    fn buying_system(
        mut commands: Commands,
        lots: Query<(), Without<LotFamily>>,
        actors: Query<&ActorFamily>,
        tasks: Query<(Entity, &Parent, &BuyLot), Added<ActiveTask>>,
    ) {
        for (entity, parent, buy) in &tasks {
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

#[derive(Clone, Component, Copy, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct BuyLot(Entity);

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
