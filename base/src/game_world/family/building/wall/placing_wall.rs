use avian3d::prelude::*;
use bevy::{
    color::palettes::css::{RED, WHITE},
    math::Vec3Swizzles,
    prelude::*,
    render::view::NoFrustumCulling,
};
use bevy_enhanced_input::prelude::*;

use super::{Wall, WallCommand, WallMaterial, WallTool};
use crate::{
    alpha_color::{AlphaColor, AlphaColorPlugin},
    common_conditions::observer_in_state,
    game_world::{
        city::ActiveCity,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::{wall::Apertures, BuildingMode},
        picking::{Clicked, Picked},
        player_camera::CameraCaster,
        spline::{dynamic_mesh::DynamicMesh, PointKind, SplineSegment},
        Layer,
    },
    ghost::Ghost,
    math::segment::Segment,
    settings::Settings,
};

pub(super) struct PlacingWallPlugin;

impl Plugin for PlacingWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_input_context::<PlacingWall>()
            .observe(Self::pick)
            .observe(Self::spawn)
            .observe(Self::delete)
            .observe(Self::cancel)
            .observe(Self::confirm)
            .add_systems(
                Update,
                Self::update_end.run_if(in_state(BuildingMode::Walls)),
            )
            .add_systems(
                PostUpdate,
                Self::update_alpha
                    .before(AlphaColorPlugin::update_materials)
                    .after(PhysicsSet::StepSimulation)
                    .run_if(in_state(BuildingMode::Walls)),
            );
    }
}

const SNAP_DELTA: f32 = 0.5;

impl PlacingWallPlugin {
    fn pick(
        trigger: Trigger<Clicked>,
        wall_tool: Option<Res<State<WallTool>>>,
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(Entity, &Parent, &SplineSegment), With<Wall>>,
    ) {
        if !observer_in_state(wall_tool, WallTool::Move) {
            return;
        }

        let Ok((entity, parent, &segment)) = walls.get(trigger.entity()) else {
            return;
        };

        const PICK_DELTA: f32 = 0.4;
        let point = trigger.event().xz();
        let kind = if segment.start.distance(point) < PICK_DELTA {
            PointKind::Start
        } else if segment.end.distance(point) < PICK_DELTA {
            PointKind::End
        } else {
            return;
        };

        info!("picking `{kind:?}` for `{entity}`");
        commands.entity(**parent).with_children(|parent| {
            parent.spawn((
                Ghost::new(entity),
                PlacingWallBundle::new(
                    PlacingWall::MovingPoint { entity, kind },
                    segment,
                    wall_material.0.clone(),
                    meshes.add(DynamicMesh::create_empty()),
                ),
            ));
        });
    }

    fn spawn(
        trigger: Trigger<Clicked>,
        wall_tool: Option<Res<State<WallTool>>>,
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(&Parent, &SplineSegment), With<Wall>>,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if !observer_in_state(wall_tool, WallTool::Create) {
            return;
        }

        let city_entity = cities.single();

        // Use an existing point if it is within the `SNAP_DELTA` distance.
        let point = trigger.event().xz();
        let snapped_point = walls
            .iter()
            .filter(|(parent, _)| ***parent == city_entity)
            .flat_map(|(_, segment)| segment.points())
            .find(|vertex| vertex.distance(point) < SNAP_DELTA)
            .unwrap_or(point);

        info!("spawning new wall");
        commands.entity(cities.single()).with_children(|parent| {
            parent.spawn(PlacingWallBundle::new(
                PlacingWall::Spawning,
                SplineSegment(Segment::splat(snapped_point)),
                wall_material.0.clone(),
                meshes.add(DynamicMesh::create_empty()),
            ));
        });
    }

    fn update_alpha(
        mut placing_walls: Query<
            (&mut AlphaColor, &CollidingEntities),
            (Changed<CollidingEntities>, With<PlacingWall>),
        >,
    ) {
        let Ok((mut alpha, colliding_entities)) = placing_walls.get_single_mut() else {
            return;
        };

        if colliding_entities.is_empty() {
            **alpha = WHITE.into();
        } else {
            **alpha = RED.into();
        };
    }

    fn update_end(
        camera_caster: CameraCaster,
        mut placing_walls: Query<(&mut SplineSegment, &Parent, &PlacingWall)>,
        walls: Query<(&Parent, &SplineSegment), (With<Wall>, Without<PlacingWall>)>,
    ) {
        let Ok((mut segment, placing_parent, &placing_wall)) = placing_walls.get_single_mut()
        else {
            return;
        };

        let Some(point) = camera_caster.intersect_ground().map(|pos| pos.xz()) else {
            return;
        };

        // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
        let vertex = walls
            .iter()
            .filter(|(parent, _)| *parent == placing_parent)
            .flat_map(|(_, segment)| segment.points())
            .find(|vertex| vertex.distance(point) < SNAP_DELTA)
            .unwrap_or(point);

        let point_kind = placing_wall.point_kind();

        trace!("updating `{point_kind:?}` to `{vertex:?}`");
        match point_kind {
            PointKind::Start => segment.start = vertex,
            PointKind::End => segment.end = vertex,
        }
    }

    fn delete(
        _trigger: Trigger<Completed<DeleteWall>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_walls: Query<(Entity, &PlacingWall, &mut SplineSegment)>,
        walls: Query<&SplineSegment, Without<PlacingWall>>,
    ) {
        if !observer_in_state(building_mode, BuildingMode::Walls) {
            return;
        }

        let Ok((placing_entity, &placing_wall, mut segment)) = placing_walls.get_single_mut()
        else {
            return;
        };

        info!("deleting wall");
        if let PlacingWall::MovingPoint { entity, .. } = placing_wall {
            // Set original segment until the deletion is confirmed.
            *segment = *walls.get(entity).expect("moving wall should exist");

            let command_id = history.push_pending(WallCommand::Delete { entity });
            commands
                .entity(placing_entity)
                .insert(PendingDespawn { command_id })
                .remove::<PlacingWall>();
        } else {
            commands.entity(placing_entity).despawn_recursive();
        }
    }

    fn cancel(
        _trigger: Trigger<Completed<CancelWall>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut commands: Commands,
        placing_walls: Query<Entity, With<PlacingWall>>,
    ) {
        if !observer_in_state(building_mode, BuildingMode::Walls) {
            return;
        }

        if let Ok(entity) = placing_walls.get_single() {
            debug!("cancelling placing");
            commands.entity(entity).despawn();
        }
    }

    fn confirm(
        _trigger: Trigger<Completed<ConfirmWall>>,
        building_mode: Option<Res<State<BuildingMode>>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_walls: Query<(Entity, &Parent, &PlacingWall, &SplineSegment)>,
    ) {
        if !observer_in_state(building_mode, BuildingMode::Walls) {
            return;
        }

        let Ok((entity, parent, &placing_wall, &segment)) = placing_walls.get_single_mut() else {
            return;
        };

        info!("configrming {placing_wall:?}");
        let command_id = match placing_wall {
            PlacingWall::Spawning => history.push_pending(WallCommand::Create {
                city_entity: **parent,
                segment: *segment,
            }),
            PlacingWall::MovingPoint { entity, kind } => {
                let point = match kind {
                    PointKind::Start => segment.start,
                    PointKind::End => segment.end,
                };
                history.push_pending(WallCommand::MovePoint {
                    entity,
                    kind,
                    point,
                })
            }
        };

        commands
            .entity(entity)
            .insert(PendingDespawn { command_id })
            .remove::<PlacingWall>();
    }
}

#[derive(Bundle)]
struct PlacingWallBundle {
    name: Name,
    placing_wall: PlacingWall,
    segment: SplineSegment,
    picked: Picked,
    alpha: AlphaColor,
    state_scoped: StateScoped<WallTool>,
    apertures: Apertures,
    collider: Collider,
    collision_layers: CollisionLayers,
    no_culling: NoFrustumCulling,
    pbr_bundle: PbrBundle,
}

impl PlacingWallBundle {
    fn new(
        placing_wall: PlacingWall,
        segment: SplineSegment,
        material: Handle<StandardMaterial>,
        mesh: Handle<Mesh>,
    ) -> Self {
        let tool = match placing_wall {
            PlacingWall::Spawning => WallTool::Create,
            PlacingWall::MovingPoint { .. } => WallTool::Move,
        };
        Self {
            name: Name::new("Placing wall"),
            placing_wall,
            segment,
            picked: Picked,
            alpha: AlphaColor(WHITE.into()),
            state_scoped: StateScoped(tool),
            apertures: Default::default(),
            collider: Default::default(),
            collision_layers: CollisionLayers::new(
                Layer::PlacingWall,
                [
                    Layer::Object,
                    Layer::PlacingObject,
                    Layer::Road,
                    Layer::PlacingRoad,
                ],
            ),
            no_culling: NoFrustumCulling,
            pbr_bundle: PbrBundle {
                material,
                mesh,
                ..Default::default()
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
enum PlacingWall {
    Spawning,
    MovingPoint { entity: Entity, kind: PointKind },
}

impl PlacingWall {
    /// Returns point kind that should be edited for this wall.
    fn point_kind(self) -> PointKind {
        match self {
            PlacingWall::Spawning => PointKind::End,
            PlacingWall::MovingPoint { entity: _, kind } => kind,
        }
    }
}

impl InputContext for PlacingWall {
    const PRIORITY: isize = 1;

    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        ctx.bind::<DeleteWall>()
            .to((&settings.keyboard.delete, GamepadButtonType::North));
        ctx.bind::<CancelWall>()
            .to((KeyCode::Escape, GamepadButtonType::East));
        ctx.bind::<ConfirmWall>()
            .to((MouseButton::Left, GamepadButtonType::South));

        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct DeleteWall;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct CancelWall;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct ConfirmWall;
