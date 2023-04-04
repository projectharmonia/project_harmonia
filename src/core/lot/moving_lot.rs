use bevy::{math::Vec3Swizzles, prelude::*};
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{LotDespawn, LotEventConfirmed, LotMove, LotTool, LotVertices};
use crate::core::{
    action::Action,
    city::{ActiveCity, CityMode},
    cursor_hover::CursorHover,
    game_state::GameState,
};

pub(super) struct MovingLotPlugin;

impl Plugin for MovingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                Self::picking_system
                    .run_if(action_just_pressed(Action::Confirm))
                    .run_if(not(any_with_component::<MovingLot>())),
                Self::movement_system,
                Self::confirmation_system.run_if(action_just_pressed(Action::Confirm)),
                Self::despawn_system.run_if(action_just_pressed(Action::Delete)),
                Self::cleanup_system.after(Self::movement_system).run_if(
                    action_just_pressed(Action::Cancel).or_else(on_event::<LotEventConfirmed>()),
                ),
            )
                .in_set(OnUpdate(GameState::City))
                .in_set(OnUpdate(CityMode::Lots))
                .in_set(OnUpdate(LotTool::Move)),
        );
    }
}

impl MovingLotPlugin {
    fn picking_system(
        mut commands: Commands,
        mut lots: Query<(Entity, &mut Visibility, &LotVertices)>,
        active_cities: Query<Entity, With<ActiveCity>>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(hover) = hovered.get_single() {
            let position = hover.0.xz();
            if let Some((entity, mut visibility, vertices)) = lots
                .iter_mut()
                .find(|(.., vertices)| vertices.contains_point(position))
            {
                *visibility = Visibility::Hidden;
                commands
                    .entity(active_cities.single())
                    .with_children(|parent| {
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
                *visibility = Visibility::Visible;
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
