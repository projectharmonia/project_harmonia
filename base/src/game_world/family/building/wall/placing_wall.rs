use avian3d::prelude::*;
use bevy::{
    color::palettes::css::{RED, WHITE},
    math::Vec3Swizzles,
    prelude::*,
    render::view::NoFrustumCulling,
};
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{Wall, WallCommand, WallMaterial, WallTool};
use crate::{
    game_world::{
        city::ActiveCity,
        commands_history::{CommandsHistory, PendingDespawn},
        family::building::{wall::Apertures, BuildingMode},
        hover::{HoverPlugin, Hovered},
        player_camera::CameraCaster,
        spline::{dynamic_mesh::DynamicMesh, PointKind, SplineSegment},
        Layer,
    },
    ghost::Ghost,
    math::segment::Segment,
    settings::Action,
};

pub(super) struct PlacingWallPlugin;

impl Plugin for PlacingWallPlugin {
    fn build(&self, app: &mut App) {
        app.observe(HoverPlugin::enable_on_remove::<PlacingWall>)
            .observe(HoverPlugin::disable_on_add::<PlacingWall>)
            .add_systems(
                Update,
                (
                    (
                        Self::spawn.run_if(in_state(WallTool::Create)),
                        Self::pick.run_if(in_state(WallTool::Move)),
                    )
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<PlacingWall>)),
                    (
                        Self::update_end,
                        Self::update_material,
                        Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                        Self::delete.run_if(action_just_pressed(Action::Delete)),
                        Self::cancel.run_if(action_just_pressed(Action::Cancel)),
                    )
                        .run_if(in_state(BuildingMode::Walls)),
                ),
            );
    }
}

const SNAP_DELTA: f32 = 0.5;

impl PlacingWallPlugin {
    fn pick(
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(Entity, &Parent, &SplineSegment, &Hovered)>,
    ) {
        let Ok((entity, parent, &segment, hovered)) = walls.get_single() else {
            return;
        };

        const PICK_DELTA: f32 = 0.4;
        let point = hovered.xz();
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
        camera_caster: CameraCaster,
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<&SplineSegment, With<Wall>>,
        cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Some(point) = camera_caster.intersect_ground().map(|point| point.xz()) {
            // Use an existing point if it is within the `SNAP_DELTA` distance.
            let point = walls
                .iter()
                .flat_map(|segment| segment.points())
                .find(|vertex| vertex.distance(point) < SNAP_DELTA)
                .unwrap_or(point);

            info!("spawning new wall");
            commands.entity(cities.single()).with_children(|parent| {
                parent.spawn(PlacingWallBundle::new(
                    PlacingWall::Spawning,
                    SplineSegment(Segment::splat(point)),
                    wall_material.0.clone(),
                    meshes.add(DynamicMesh::create_empty()),
                ));
            });
        }
    }

    fn update_material(
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut placing_walls: Query<
            (&mut Handle<StandardMaterial>, &CollidingEntities),
            (Changed<CollidingEntities>, With<PlacingWall>),
        >,
    ) {
        if let Ok((mut material_handle, colliding_entities)) = placing_walls.get_single_mut() {
            let mut material = materials
                .get(&*material_handle)
                .cloned()
                .expect("material should be preloaded");

            let color = if colliding_entities.is_empty() {
                WHITE.into()
            } else {
                RED.into()
            };
            debug!("changing base color to `{color:?}`");

            material.alpha_mode = AlphaMode::Add;
            material.base_color = color;

            *material_handle = materials.add(material);
        }
    }

    fn update_end(
        camera_caster: CameraCaster,
        mut placing_walls: Query<(&mut SplineSegment, &Parent, &PlacingWall)>,
        walls: Query<&SplineSegment, (With<Wall>, Without<PlacingWall>)>,
        children: Query<&Children>,
    ) {
        if let Ok((mut segment, parent, &placing_wall)) = placing_walls.get_single_mut() {
            if let Some(point) = camera_caster.intersect_ground().map(|pos| pos.xz()) {
                let children = children.get(**parent).unwrap();

                // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
                let vertex = walls
                    .iter_many(children)
                    .flat_map(|segment| segment.points())
                    .find(|vertex| vertex.distance(point) < SNAP_DELTA)
                    .unwrap_or(point);

                let point_kind = placing_wall.point_kind();

                trace!("updating `{point_kind:?}` to `{vertex:?}`");
                match point_kind {
                    PointKind::Start => segment.start = vertex,
                    PointKind::End => segment.end = vertex,
                }
            }
        }
    }

    fn confirm(
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_walls: Query<(Entity, &Parent, &PlacingWall, &SplineSegment)>,
    ) {
        if let Ok((entity, parent, &placing_wall, &segment)) = placing_walls.get_single_mut() {
            info!("configrming {placing_wall:?}");

            let id = match placing_wall {
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
                .insert(PendingDespawn(id))
                .remove::<PlacingWall>();
        }
    }

    fn delete(
        mut commands: Commands,
        mut history: CommandsHistory,
        mut placing_walls: Query<(Entity, &PlacingWall, &mut SplineSegment)>,
        walls: Query<&SplineSegment, Without<PlacingWall>>,
    ) {
        if let Ok((placing_entity, &placing_wall, mut segment)) = placing_walls.get_single_mut() {
            info!("deleting wall");
            if let PlacingWall::MovingPoint { entity, .. } = placing_wall {
                // Set original segment until the deletion is confirmed.
                *segment = *walls.get(entity).expect("moving wall should exist");

                let id = history.push_pending(WallCommand::Delete { entity });
                commands
                    .entity(placing_entity)
                    .insert(PendingDespawn(id))
                    .remove::<PlacingWall>();
            } else {
                commands.entity(placing_entity).despawn_recursive();
            }
        }
    }

    fn cancel(mut commands: Commands, placing_walls: Query<Entity, With<PlacingWall>>) {
        if let Ok(entity) = placing_walls.get_single() {
            debug!("cancelling placing");
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Bundle)]
struct PlacingWallBundle {
    name: Name,
    placing_wall: PlacingWall,
    segment: SplineSegment,
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
            state_scoped: StateScoped(tool),
            apertures: Default::default(),
            collider: Default::default(),
            // TODO: collide with regular walls.
            collision_layers: CollisionLayers::new(Layer::PlacingWall, [Layer::Object]),
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
pub enum PlacingWall {
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
