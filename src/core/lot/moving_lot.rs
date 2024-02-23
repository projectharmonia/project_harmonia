use bevy::{math::Vec3Swizzles, prelude::*};
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{LotDespawn, LotEventConfirmed, LotMove, LotTool, LotVertices};
use crate::core::{
    action::Action, city::CityMode, cursor_hover::CursorHover, game_state::GameState,
};

pub(super) struct MovingLotPlugin;

impl Plugin for MovingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                Self::picking_system
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(not(any_with_component::<MovingLot>)),
                Self::movement_system,
                Self::confirmation_system.run_if(action_just_pressed(Action::Confirm)),
                Self::despawn_system.run_if(action_just_pressed(Action::Delete)),
                Self::cancel_system.run_if(
                    action_just_pressed(Action::Cancel).or_else(on_event::<LotEventConfirmed>()),
                ),
            )
                .run_if(in_state(GameState::City))
                .run_if(in_state(CityMode::Lots))
                .run_if(in_state(LotTool::Move)),
        )
        .add_systems(
            PostUpdate,
            Self::cleanup_system
                .run_if(in_state(GameState::City))
                .run_if(in_state(CityMode::Lots))
                .run_if(in_state(LotTool::Move)),
        );
    }
}

impl MovingLotPlugin {
    fn picking_system(
        mut commands: Commands,
        lots: Query<(Entity, &Parent, &LotVertices)>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(hover) = hovered.get_single() {
            if let Some((entity, parent, vertices)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(hover.xz()))
            {
                commands.entity(**parent).with_children(|parent| {
                    parent.spawn((
                        vertices.clone(),
                        MovingLot {
                            entity,
                            offset: hover.0,
                        },
                    ));
                });
            }
        }
    }

    fn movement_system(
        mut moving_lots: Query<(&mut Transform, &MovingLot)>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok((mut transform, moving_lot)) = moving_lots.get_single_mut() {
            if let Ok(hover) = hovered.get_single() {
                transform.translation = hover.0 - moving_lot.offset;
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
            });
        }
    }

    fn despawn_system(mut despawn_events: EventWriter<LotDespawn>, moving_lots: Query<&MovingLot>) {
        if let Ok(moving_lot) = moving_lots.get_single() {
            despawn_events.send(LotDespawn(moving_lot.entity));
        }
    }

    fn cancel_system(mut commands: Commands, mut moving_lots: Query<Entity, With<MovingLot>>) {
        if let Ok(entity) = moving_lots.get_single_mut() {
            commands.entity(entity).despawn();
        }
    }

    fn cleanup_system(mut commands: Commands, mut moving_lots: Query<(Entity, &MovingLot)>) {
        if let Ok((entity, moving_lot)) = moving_lots.get_single_mut() {
            if commands.get_entity(moving_lot.entity).is_none() {
                commands.entity(entity).despawn();
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
