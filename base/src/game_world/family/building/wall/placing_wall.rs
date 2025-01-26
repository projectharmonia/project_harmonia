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
    dynamic_mesh::DynamicMesh,
    game_world::{
        city::ActiveCity,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::{wall::Apertures, BuildingMode},
        segment::{
            placing_segment::{CancelSegment, ConfirmSegment, DeleteSegment, PlacingSegment},
            ruler::Ruler,
            PointKind, Segment,
        },
        Layer,
    },
    ghost::Ghost,
};

pub(super) struct PlacingWallPlugin;

impl Plugin for PlacingWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::pick.never_param_warn())
            .add_observer(Self::spawn.never_param_warn())
            .add_observer(Self::delete)
            .add_observer(Self::cancel.never_param_warn())
            .add_observer(Self::confirm)
            .add_systems(
                PostUpdate,
                Self::update_alpha
                    .never_param_warn()
                    .before(AlphaColorPlugin::update_materials)
                    .run_if(in_state(BuildingMode::Walls)),
            );
    }
}

const SNAP_DELTA: f32 = 0.5;

impl PlacingWallPlugin {
    fn pick(
        mut trigger: Trigger<Pointer<Click>>,
        wall_tool: Res<State<WallTool>>,
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(Entity, &Parent, &Segment), With<Wall>>,
        placing_walls: Query<(), With<PlacingWall>>,
    ) {
        if trigger.button != PointerButton::Primary {
            return;
        }
        if *wall_tool != WallTool::Move {
            return;
        }
        if !placing_walls.is_empty() {
            return;
        }
        let Ok((entity, parent, &segment)) = walls.get(trigger.entity()) else {
            return;
        };
        trigger.propagate(false);

        const PICK_DELTA: f32 = 0.4;
        let point = trigger.hit.position.unwrap();
        let point_kind = if segment.start.distance(point.xz()) < PICK_DELTA {
            PointKind::Start
        } else if segment.end.distance(point.xz()) < PICK_DELTA {
            PointKind::End
        } else {
            return;
        };

        info!("picking `{point_kind:?}` for `{entity}`");
        commands.entity(**parent).with_children(|parent| {
            parent.spawn((
                Ghost::new(entity),
                PlacingWall::EditingPoint { entity },
                WallTool::Move,
                segment,
                PlacingSegment {
                    point_kind,
                    snap_offset: 0.5,
                },
                wall_material.0.clone(),
                Mesh3d(meshes.add(DynamicMesh::create_empty())),
            ));
        });
    }

    fn spawn(
        mut trigger: Trigger<Pointer<Click>>,
        wall_tool: Res<State<WallTool>>,
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(&Parent, &Segment), With<Wall>>,
        city_entity: Single<Entity, With<ActiveCity>>,
        placing_walls: Query<(), With<PlacingWall>>,
    ) {
        if trigger.button != PointerButton::Primary {
            return;
        }
        if *wall_tool != WallTool::Create {
            return;
        }
        if !placing_walls.is_empty() {
            return;
        }
        let Some(point) = trigger.hit.position else {
            // Consider only world clicking.
            return;
        };

        trigger.propagate(false);

        // Use an existing point if it is within the `SNAP_DELTA` distance.
        let snapped_point = walls
            .iter()
            .filter(|(parent, _)| ***parent == *city_entity)
            .flat_map(|(_, segment)| segment.points())
            .find(|vertex| vertex.distance(point.xz()) < SNAP_DELTA)
            .unwrap_or(point.xz());

        info!("spawning new wall");
        commands.entity(*city_entity).with_children(|parent| {
            parent.spawn((
                PlacingWall::Spawning,
                WallTool::Create,
                Segment::splat(snapped_point),
                PlacingSegment {
                    point_kind: PointKind::End,
                    snap_offset: 0.5,
                },
                wall_material.0.clone(),
                Mesh3d(meshes.add(DynamicMesh::create_empty())),
            ));
        });
    }

    fn update_alpha(
        placing_wall: Single<
            (&mut AlphaColor, &CollidingEntities),
            (Changed<CollidingEntities>, With<PlacingWall>),
        >,
    ) {
        let (mut alpha, colliding_entities) = placing_wall.into_inner();
        if colliding_entities.is_empty() {
            **alpha = WHITE.into();
        } else {
            **alpha = RED.into();
        };
    }

    fn delete(
        trigger: Trigger<Completed<DeleteSegment>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        placing_wall: Single<(&PlacingWall, &mut Segment)>,
        walls: Query<&Segment, Without<PlacingWall>>,
    ) {
        let (&placing_wall, mut segment) = placing_wall.into_inner();

        info!("deleting wall");
        if let PlacingWall::EditingPoint { entity } = placing_wall {
            // Set original segment until the deletion is confirmed.
            *segment = *walls.get(entity).expect("moving wall should exist");

            let command_id = history.push_pending(WallCommand::Delete { entity });
            commands
                .entity(trigger.entity())
                .insert(PendingDespawn { command_id })
                .remove::<PlacingWall>();
        } else {
            commands.entity(trigger.entity()).despawn_recursive();
        }
    }

    fn cancel(trigger: Trigger<Completed<CancelSegment>>, mut commands: Commands) {
        debug!("cancelling placing");
        commands.entity(trigger.entity()).despawn_recursive();
    }

    fn confirm(
        trigger: Trigger<Completed<ConfirmSegment>>,
        mut commands: Commands,
        mut history: CommandsHistory,
        placing_wall: Single<(&Parent, &PlacingWall, &Segment, &PlacingSegment)>,
    ) {
        let (parent, &placing_wall, &segment, placing_segment) = *placing_wall;

        info!("configrming {placing_wall:?}");
        let command_id = match placing_wall {
            PlacingWall::Spawning => history.push_pending(WallCommand::Create {
                city_entity: **parent,
                segment,
            }),
            PlacingWall::EditingPoint { entity } => {
                let point = segment.point(placing_segment.point_kind);
                history.push_pending(WallCommand::EditPoint {
                    entity,
                    kind: placing_segment.point_kind,
                    point,
                })
            }
        };

        commands
            .entity(trigger.entity())
            .insert(PendingDespawn { command_id })
            .remove::<PlacingWall>();
    }
}

#[derive(Debug, Clone, Copy, Component)]
#[require(
    Name(|| Name::new("Placing wall")),
    Mesh3d,
    MeshMaterial3d::<StandardMaterial>,
    // Looks like AABB is not recalculated when we edit the mesh.
    // But we don't need to cull currently placed wall anyway.
    NoFrustumCulling,
    Ruler,
    AlphaColor(|| AlphaColor(WHITE.into())),
    Apertures,
    Collider,
    CollisionLayers(|| CollisionLayers::new(
        Layer::PlacingWall,
        [
            Layer::Object,
            Layer::PlacingObject,
            Layer::Road,
            Layer::PlacingRoad,
        ],
    ))
)]
enum PlacingWall {
    Spawning,
    EditingPoint { entity: Entity },
}
