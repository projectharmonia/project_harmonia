pub mod placing_wall;
mod triangulator;
pub(crate) mod wall_mesh;

use avian3d::prelude::*;
use bevy::{ecs::entity::MapEntities, prelude::*, render::view::NoFrustumCulling};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter};

use super::BuildingMode;
use crate::{
    core::GameState,
    dynamic_mesh::DynamicMesh,
    game_world::{
        commands_history::{
            CommandConfirmation, CommandId, CommandRequest, ConfirmableCommand, EntityRecorder,
            PendingCommand,
        },
        navigation::Obstacle,
        segment::{PointKind, Segment, SegmentConnections, SegmentPlugin},
        Layer,
    },
};
use placing_wall::PlacingWallPlugin;
use triangulator::Triangulator;

pub(crate) struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlacingWallPlugin)
            .add_sub_state::<WallTool>()
            .enable_state_scoped_entities::<WallTool>()
            .init_resource::<WallMaterial>()
            .register_type::<Wall>()
            .replicate::<Wall>()
            .add_mapped_client_event::<CommandRequest<WallCommand>>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::apply_command
                        .run_if(server_or_singleplayer)
                        .before(ServerSet::StoreHierarchy),
                    Self::update_meshes.after(SegmentPlugin::update_connections),
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl WallPlugin {
    fn init(
        mut commands: Commands,
        wall_material: Res<WallMaterial>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<Entity, (With<Wall>, Without<Handle<Mesh>>)>,
    ) {
        for entity in &walls {
            debug!("initializing wall `{entity}`");

            commands.entity(entity).insert((
                Name::new("Wall"),
                Apertures::default(),
                Collider::default(),
                CollisionLayers::new(
                    Layer::Wall,
                    [
                        Layer::Object,
                        Layer::PlacingObject,
                        Layer::Road,
                        Layer::PlacingRoad,
                    ],
                ),
                NoFrustumCulling,
                Obstacle,
                PbrBundle {
                    material: wall_material.0.clone(),
                    mesh: meshes.add(DynamicMesh::create_empty()),
                    ..Default::default()
                },
            ));
        }
    }

    pub(crate) fn update_meshes(
        mut triangulator: Local<Triangulator>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut changed_walls: Query<
            (
                &Handle<Mesh>,
                Ref<Segment>,
                &SegmentConnections,
                &mut Apertures,
                &mut Collider,
            ),
            Or<(Changed<SegmentConnections>, Changed<Apertures>)>,
        >,
    ) {
        for (mesh_handle, segment, connections, mut apertures, mut collider) in &mut changed_walls {
            let mesh = meshes
                .get_mut(mesh_handle)
                .expect("wall handles should be valid");

            trace!("regenerating wall mesh");
            let mut dyn_mesh = DynamicMesh::take(mesh);
            wall_mesh::generate(
                &mut dyn_mesh,
                *segment,
                connections,
                &apertures,
                &mut triangulator,
            );
            dyn_mesh.apply(mesh);

            if apertures.collision_outdated || segment.is_changed() || collider.is_added() {
                trace!("regenerating wall collision");
                *collider = wall_mesh::generate_collider(*segment, &apertures);
                apertures.collision_outdated = false;
            }
        }
    }

    fn apply_command(
        mut commands: Commands,
        mut request_events: EventReader<FromClient<CommandRequest<WallCommand>>>,
        mut confirm_events: EventWriter<ToClients<CommandConfirmation>>,
        mut walls: Query<&mut Segment, With<Wall>>,
    ) {
        for FromClient { client_id, event } in request_events.read().copied() {
            // TODO: validate if command can be applied.
            let mut confirmation = CommandConfirmation::new(event.id);
            match event.command {
                WallCommand::Create {
                    city_entity,
                    segment,
                } => {
                    info!("`{client_id:?}` creates wall");
                    commands.entity(city_entity).with_children(|parent| {
                        let entity = parent.spawn(WallBundle::new(segment)).id();
                        confirmation.entity = Some(entity);
                    });
                }
                WallCommand::EditPoint {
                    entity,
                    kind,
                    point,
                } => match walls.get_mut(entity) {
                    Ok(mut segment) => {
                        info!("`{client_id:?}` edits `{kind:?}` for wall `{entity}`");
                        segment.set_point(kind, point);
                    }
                    Err(e) => error!("unable to move wall `{entity}`: {e}"),
                },
                WallCommand::Delete { entity } => {
                    info!("`{client_id:?}` removes wall `{entity}`");
                    commands.entity(entity).despawn();
                }
            }

            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: confirmation,
            });
        }
    }
}

#[derive(Resource)]
struct WallMaterial(Handle<StandardMaterial>);

impl FromWorld for WallMaterial {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        Self(asset_server.load("base/walls/brick/brick.ron"))
    }
}

#[derive(
    Clone, Component, Copy, Debug, Default, Display, EnumIter, Eq, Hash, PartialEq, SubStates,
)]
#[source(BuildingMode = BuildingMode::Walls)]
pub enum WallTool {
    #[default]
    Create,
    Move,
}

impl WallTool {
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Create => "✏",
            Self::Move => "↔",
        }
    }
}

#[derive(Bundle)]
struct WallBundle {
    wall: Wall,
    segment: Segment,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl WallBundle {
    fn new(segment: Segment) -> Self {
        Self {
            wall: Wall,
            segment,
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

#[derive(Component, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct Wall;

/// Dynamically updated component with precalculated apertures for wall objects.
///
/// Apertures are sorted by distance to the wall starting point.
#[derive(Component, Default)]
pub(crate) struct Apertures {
    apertures: Vec<Aperture>,
    collision_outdated: bool,
}

impl Apertures {
    /// Returns iterator over all apertures.
    fn iter(&self) -> impl Iterator<Item = &Aperture> {
        self.apertures.iter()
    }

    /// Inserts a new aperture in sorted order.
    pub(crate) fn insert(&mut self, aperture: Aperture) {
        let index = self
            .apertures
            .binary_search_by(|other| other.distance.total_cmp(&aperture.distance))
            .expect_err("apertures shouldn't have duplicates");

        if !aperture.placing_object && !aperture.hole {
            self.collision_outdated = true;
        }
        self.apertures.insert(index, aperture);
    }

    /// Returns aperture by its index.
    pub(crate) fn remove(&mut self, entity: Entity) -> Aperture {
        let index = self
            .iter()
            .position(|aperture| aperture.object_entity == entity)
            .unwrap_or_else(|| panic!("entity `{entity}` should be present in apertures"));

        let aperture = self.apertures.remove(index);
        if !aperture.placing_object && !aperture.hole {
            self.collision_outdated = true;
        }
        aperture
    }
}

pub(crate) struct Aperture {
    /// The entity that cut this aperture.
    pub(crate) object_entity: Entity,

    /// Position of the aperture.
    pub(crate) translation: Vec3,

    /// Distance to the beginning of the wall.
    ///
    /// Used for sorting in [`Apertures`].
    pub(crate) distance: f32,

    /// Positions relative to the coordinate origin at which the aperture is cut in 2D space.
    pub(crate) cutout: Vec<Vec2>,

    /// Indicates if the aperture is hole (like a window) or clipping (like a door or arch).
    pub(crate) hole: bool,

    /// Indicates if the aperture caused by an object that has not yet been placed.
    pub(crate) placing_object: bool,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
enum WallCommand {
    Create {
        city_entity: Entity,
        segment: Segment,
    },
    EditPoint {
        entity: Entity,
        kind: PointKind,
        point: Vec2,
    },
    Delete {
        entity: Entity,
    },
}

impl PendingCommand for WallCommand {
    fn apply(
        self: Box<Self>,
        id: CommandId,
        mut recorder: EntityRecorder,
        world: &mut World,
    ) -> Box<dyn ConfirmableCommand> {
        let reverse_command = match *self {
            Self::Create { .. } => Self::Delete {
                // Correct entity will be set after the server confirmation.
                entity: Entity::PLACEHOLDER,
            },
            Self::EditPoint { entity, kind, .. } => {
                let segment = world.get::<Segment>(entity).unwrap();
                let point = segment.point(kind);
                Self::EditPoint {
                    entity,
                    kind,
                    point,
                }
            }
            Self::Delete { entity } => {
                recorder.record(entity);
                let entity = world.entity(entity);
                let segment = *entity.get::<Segment>().unwrap();
                let city_entity = **entity.get::<Parent>().unwrap();
                Self::Create {
                    city_entity,
                    segment,
                }
            }
        };

        world.send_event(CommandRequest { id, command: *self });

        Box::new(reverse_command)
    }
}

impl ConfirmableCommand for WallCommand {
    fn confirm(
        mut self: Box<Self>,
        mut recorder: EntityRecorder,
        confirmation: CommandConfirmation,
    ) -> Box<dyn PendingCommand> {
        if let Self::Delete { entity } = &mut *self {
            *entity = confirmation
                .entity
                .expect("confirmation for wall creation should contain an entity");
            recorder.record(*entity);
        }

        self
    }
}

impl MapEntities for WallCommand {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        match self {
            Self::Create { .. } => (),
            Self::EditPoint { entity, .. } => *entity = entity_mapper.map_entity(*entity),
            Self::Delete { entity } => *entity = entity_mapper.map_entity(*entity),
        };
    }
}
