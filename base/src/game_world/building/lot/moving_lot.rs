use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_replicon::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{LotDelete, LotEventConfirmed, LotMove, LotTool, LotVertices, UnconfirmedLot};
use crate::{
    game_world::{city::CityMode, player_camera::CameraCaster},
    settings::Action,
};

pub(super) struct MovingLotPlugin;

impl Plugin for MovingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(CityMode::Lots), Self::end_creating)
            .add_systems(OnExit(LotTool::Move), Self::end_creating)
            .add_systems(
                PreUpdate,
                Self::end_creating
                    .after(ClientSet::Receive)
                    .run_if(in_state(LotTool::Move))
                    .run_if(on_event::<LotEventConfirmed>()),
            )
            .add_systems(
                Update,
                (
                    Self::pick
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<MovingLot>)),
                    Self::apply_movement,
                    Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                    Self::delete.run_if(action_just_pressed(Action::Delete)),
                    Self::end_creating.run_if(action_just_pressed(Action::Cancel)),
                )
                    .run_if(in_state(LotTool::Move)),
            )
            .add_systems(
                PostUpdate,
                Self::cleanup_despawned.run_if(in_state(LotTool::Move)),
            );
    }
}

impl MovingLotPlugin {
    fn pick(
        camera_caster: CameraCaster,
        mut commands: Commands,
        lots: Query<(Entity, &Parent, &LotVertices)>,
    ) {
        if let Some(point) = camera_caster.intersect_ground() {
            if let Some((entity, parent, vertices)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(point.xz()))
            {
                info!("picking lot `{entity:?}`");
                commands.entity(**parent).with_children(|parent| {
                    parent.spawn((
                        vertices.clone(),
                        MovingLot {
                            entity,
                            offset: point,
                        },
                    ));
                });
            }
        }
    }

    fn apply_movement(
        camera_caster: CameraCaster,
        mut moving_lots: Query<(&mut Transform, &MovingLot), Without<UnconfirmedLot>>,
    ) {
        if let Ok((mut transform, moving_lot)) = moving_lots.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground() {
                transform.translation = point - moving_lot.offset;
            }
        }
    }

    fn confirm(
        mut move_events: EventWriter<LotMove>,
        mut moving_lots: Query<(&mut Transform, &MovingLot), Without<UnconfirmedLot>>,
    ) {
        if let Ok((transform, moving_lot)) = moving_lots.get_single_mut() {
            info!("confirming lot movement");
            move_events.send(LotMove {
                entity: moving_lot.entity,
                offset: transform.translation.xz(),
            });
        }
    }

    fn delete(
        mut delete: EventWriter<LotDelete>,
        moving_lots: Query<&MovingLot, Without<UnconfirmedLot>>,
    ) {
        if let Ok(moving_lot) = moving_lots.get_single() {
            info!("deleting picked lot");
            delete.send(LotDelete(moving_lot.entity));
        }
    }

    fn end_creating(mut commands: Commands, mut moving_lots: Query<Entity, With<MovingLot>>) {
        if let Ok(entity) = moving_lots.get_single_mut() {
            info!("ending lot movement");
            commands.entity(entity).despawn();
        }
    }

    fn cleanup_despawned(mut commands: Commands, mut moving_lots: Query<(Entity, &MovingLot)>) {
        if let Ok((entity, moving_lot)) = moving_lots.get_single_mut() {
            if commands.get_entity(moving_lot.entity).is_none() {
                info!(
                    "cancelling movement for despawned lot `{:?}`",
                    moving_lot.entity
                );
                commands.entity(entity).despawn();
            }
        }
    }
}

#[derive(Component)]
pub struct MovingLot {
    /// The entity of the lot for which the movement is performed.
    entity: Entity,
    /// Contains the offset of the cursor position to the position of the object when it was picked.
    offset: Vec3,
}
