use bevy::{math::Vec3Swizzles, prelude::*};
use bevy_enhanced_input::prelude::*;

use super::{LotCreate, LotTool, LotVertices, UnconfirmedLot};
use crate::{
    common_conditions::observer_in_state,
    game_world::{city::ActiveCity, picking::Clicked, player_camera::CameraCaster},
};

pub(super) struct CreatingLotPlugin;

impl Plugin for CreatingLotPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<CreatingLot>()
            .observe(Self::start)
            .observe(Self::cancel)
            .observe(Self::confirm)
            .add_systems(
                Update,
                Self::set_vertex_position.run_if(in_state(LotTool::Create)),
            );
    }
}

impl CreatingLotPlugin {
    fn start(
        trigger: Trigger<Clicked>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut commands: Commands,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Create) {
            return;
        }

        info!("starting placing lot");
        // Spawn with two the same vertices because we edit the last one on cursor movement.
        commands.entity(cities.single()).with_children(|parent| {
            parent.spawn((
                StateScoped(LotTool::Create),
                LotVertices(vec![trigger.event().xz(); 2].into()),
                CreatingLot,
            ));
        });
    }

    fn set_vertex_position(
        camera_caster: CameraCaster,
        mut creating_lots: Query<&mut LotVertices, (With<CreatingLot>, Without<UnconfirmedLot>)>,
    ) {
        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground().map(|hover| hover.xz()) {
                let first_vertex = *lot_vertices
                    .first()
                    .expect("vertices should have at least 2 vertices");
                let last_vertex = lot_vertices.last_mut().unwrap();

                const SNAP_DELTA: f32 = 0.1;
                let delta = first_vertex - point;
                if delta.x.abs() <= SNAP_DELTA && delta.y.abs() <= SNAP_DELTA {
                    trace!("snapping vertex position to last vertex `{last_vertex:?}`");
                    *last_vertex = first_vertex;
                } else {
                    trace!("updating vertex position to `{point:?}`");
                    *last_vertex = point;
                }
            }
        }
    }

    fn cancel(
        _trigger: Trigger<Completed<CancelLot>>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut commands: Commands,
        creating_lots: Query<Entity, With<CreatingLot>>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Create) {
            return;
        }

        if let Ok(entity) = creating_lots.get_single() {
            info!("ending lot creation");
            commands.entity(entity).despawn();
        }
    }

    fn confirm(
        _trigger: Trigger<Completed<ConfirmLot>>,
        lot_tool: Option<Res<State<LotTool>>>,
        mut create_events: EventWriter<LotCreate>,
        mut creating_lots: Query<&mut LotVertices, (With<CreatingLot>, Without<UnconfirmedLot>)>,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if !observer_in_state(lot_tool, LotTool::Create) {
            return;
        }

        if let Ok(mut lot_vertices) = creating_lots.get_single_mut() {
            let first_vertex = *lot_vertices
                .first()
                .expect("vertices should have at least 2 vertices");
            let last_vertex = *lot_vertices.last().unwrap();
            if first_vertex == last_vertex {
                info!("confirming lot creation");
                create_events.send(LotCreate {
                    polygon: lot_vertices.0.clone(),
                    city_entity: cities.single(),
                });
            } else {
                info!("confirming lot point");
                lot_vertices.push(last_vertex);
            }
        }
    }
}

#[derive(Component)]
struct CreatingLot;

impl InputContext for CreatingLot {
    const PRIORITY: isize = 1;

    fn context_instance(_world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();

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
struct CancelLot;

#[derive(Debug, InputAction)]
#[input_action(dim = Bool)]
struct ConfirmLot;
