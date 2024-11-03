use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_enhanced_input::prelude::*;

use super::{LotDelete, LotMove, LotTool, LotVertices, UnconfirmedLot};
use crate::{
    common_conditions::observer_in_state,
    game_world::{picking::Clicked, player_camera::CameraCaster},
    settings::Settings,
};

pub(super) struct MovingLotPlugin;

impl Plugin for MovingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<MovingLot>()
            .observe(Self::pick)
            .observe(Self::delete)
            .observe(Self::cancel)
            .observe(Self::confirm)
            .add_systems(Update, Self::apply_movement.run_if(in_state(LotTool::Move)))
            .add_systems(
                PostUpdate,
                Self::cleanup_despawned.run_if(in_state(LotTool::Move)),
            );
    }
}

impl MovingLotPlugin {
    fn pick(
        trigger: Trigger<Clicked>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut commands: Commands,
        lots: Query<(Entity, &Parent, &LotVertices)>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Move) {
            return;
        }

        let point = trigger.event().xz();
        if let Some((entity, parent, vertices)) = lots
            .iter()
            .find(|(.., vertices)| vertices.contains_point(point))
        {
            info!("picking lot `{entity}`");
            commands.entity(**parent).with_children(|parent| {
                parent.spawn((
                    StateScoped(LotTool::Move),
                    vertices.clone(),
                    MovingLot {
                        entity,
                        offset: **trigger.event(),
                    },
                ));
            });
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

    fn delete(
        _trigger: Trigger<Completed<DeleteLot>>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut delete: EventWriter<LotDelete>,
        moving_lots: Query<&MovingLot, Without<UnconfirmedLot>>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Move) {
            return;
        }

        if let Ok(moving_lot) = moving_lots.get_single() {
            info!("deleting picked lot");
            delete.send(LotDelete(moving_lot.entity));
        }
    }

    fn cancel(
        _trigger: Trigger<Completed<CancelLot>>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut commands: Commands,
        mut moving_lots: Query<Entity, With<MovingLot>>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Move) {
            return;
        }

        if let Ok(entity) = moving_lots.get_single_mut() {
            info!("ending lot movement");
            commands.entity(entity).despawn();
        }
    }

    fn confirm(
        _trigger: Trigger<Completed<ConfirmLot>>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut move_events: EventWriter<LotMove>,
        mut moving_lots: Query<(&mut Transform, &MovingLot), Without<UnconfirmedLot>>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Move) {
            return;
        }

        if let Ok((transform, moving_lot)) = moving_lots.get_single_mut() {
            info!("confirming lot movement");
            move_events.send(LotMove {
                entity: moving_lot.entity,
                offset: transform.translation.xz(),
            });
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
struct MovingLot {
    /// The entity of the lot for which the movement is performed.
    entity: Entity,
    /// Contains the offset of the cursor position to the position of the object when it was picked.
    offset: Vec3,
}

impl InputContext for MovingLot {
    const PRIORITY: isize = 1;

    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        let delete = ctx.bind::<DeleteLot>().with(GamepadButtonType::North);
        for &key in &settings.keyboard.delete {
            delete.with(key);
        }

        ctx.bind::<CancelLot>()
            .with(KeyCode::Escape)
            .with(GamepadButtonType::East);

        ctx.bind::<ConfirmLot>()
            .with(MouseButton::Left)
            .with(GamepadButtonType::South);

        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(dim = Bool)]
struct DeleteLot;

#[derive(Debug, InputAction)]
#[input_action(dim = Bool)]
struct CancelLot;

#[derive(Debug, InputAction)]
#[input_action(dim = Bool)]
struct ConfirmLot;
