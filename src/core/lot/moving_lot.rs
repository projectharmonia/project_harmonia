use bevy::{math::Vec3Swizzles, prelude::*};
use iyes_loopless::prelude::*;

use super::{LotDespawn, LotEventConfirmed, LotMove, LotTool, LotVertices};
use crate::core::{
    action::{self, Action},
    city::{ActiveCity, CityMode},
    condition,
    game_state::GameState,
    ground::GroundPlugin,
};

#[derive(SystemLabel)]
enum MovingLotPluginSystem {
    Movement,
}

pub(super) struct MovingLotPlugin;

impl Plugin for MovingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            CoreStage::PostUpdate, // To flush spawning.
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::picking_system)
                .run_if(action::just_pressed(Action::Confirm))
                .run_if_not(condition::any_component_exists::<MovingLot>())
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Move),
        )
        .add_system(
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::movement_system)
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Move)
                .label(MovingLotPluginSystem::Movement),
        )
        .add_system(
            Self::confirmation_system
                .run_if(action::just_pressed(Action::Confirm))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Move),
        )
        .add_system(
            Self::despawn_system
                .run_if(action::just_pressed(Action::Delete))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Move),
        )
        .add_system(
            Self::cleanup_system
                .run_if(action::just_pressed(Action::Cancel))
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Move)
                .after(MovingLotPluginSystem::Movement),
        )
        .add_system(
            Self::cleanup_system
                .run_on_event::<LotEventConfirmed>()
                .run_in_state(GameState::City)
                .run_in_state(CityMode::Lots)
                .run_in_state(LotTool::Move)
                .after(MovingLotPluginSystem::Movement),
        );
    }
}

impl MovingLotPlugin {
    fn picking_system(
        In(position): In<Option<Vec2>>,
        mut commands: Commands,
        mut lots: Query<(Entity, &mut Visibility, &LotVertices)>,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Some(position) = position {
            if let Some((entity, mut visibility, vertices)) = lots
                .iter_mut()
                .find(|(.., vertices)| vertices.contains_point(position))
            {
                visibility.is_visible = false;
                commands
                    .entity(active_cities.single())
                    .with_children(|parent| {
                        parent.spawn((
                            vertices.clone(),
                            MovingLot {
                                entity,
                                offset: Vec3::new(position.x, 0.0, position.y),
                            },
                        ));
                    });
            }
        }
    }

    fn movement_system(
        In(position): In<Option<Vec2>>,
        mut moving_lots: Query<(&mut Transform, &MovingLot)>,
    ) {
        if let Ok((mut transform, moving_lot)) = moving_lots.get_single_mut() {
            if let Some(position) = position {
                transform.translation = Vec3::new(position.x, 0.0, position.y) - moving_lot.offset;
            }
        }
    }

    fn confirmation_system(
        mut move_events: EventWriter<LotMove>,
        mut moving_lots: Query<(&mut Transform, &MovingLot)>,
    ) {
        if let Ok((transform, moving_lot)) = moving_lots.get_single_mut() {
            move_events.send(LotMove {
                entity: moving_lot.entity,
                offset: transform.translation.xz(),
            })
        }
    }

    fn despawn_system(mut despawn_events: EventWriter<LotDespawn>, moving_lots: Query<&MovingLot>) {
        if let Ok(moving_lot) = moving_lots.get_single() {
            despawn_events.send(LotDespawn(moving_lot.entity));
        }
    }

    fn cleanup_system(
        mut commands: Commands,
        mut visibility: Query<&mut Visibility>,
        mut moving_lots: Query<(Entity, &MovingLot)>,
    ) {
        if let Ok((entity, moving_lot)) = moving_lots.get_single_mut() {
            commands.entity(entity).despawn();
            // Lot could be invalid in case of removal.
            if let Ok(mut visibility) = visibility.get_mut(moving_lot.entity) {
                visibility.is_visible = true;
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct MovingLot {
    /// The entity of the lot for which the movement is performed.
    entity: Entity,
    /// Contains the offset of the cursor position to the position of the object when it was picked.
    offset: Vec3,
}
