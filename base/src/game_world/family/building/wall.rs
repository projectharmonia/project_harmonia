pub mod placing_wall;
pub(crate) mod wall_mesh;

use bevy::{ecs::entity::MapEntities, prelude::*, render::view::NoFrustumCulling};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use oxidized_navigation::NavMeshAffector;
use serde::{Deserialize, Serialize};

use crate::{
    core::GameState,
    game_world::{
        spline::{dynamic_mesh::DynamicMesh, SplineConnections, SplinePlugin, SplineSegment},
        Layer,
    },
    math::triangulator::Triangulator,
};
use placing_wall::{CreatingWall, PlacingWallPlugin};

pub(crate) struct WallPlugin;

impl Plugin for WallPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlacingWallPlugin)
            .register_type::<Wall>()
            .replicate::<Wall>()
            .add_mapped_client_event::<WallCreate>(ChannelKind::Unordered)
            .add_server_event::<WallCreateConfirmed>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                (
                    Self::create
                        .run_if(has_authority)
                        .before(ServerSet::StoreHierarchy),
                    Self::update_meshes.after(SplinePlugin::update_connections),
                )
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

impl WallPlugin {
    fn init(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut meshes: ResMut<Assets<Mesh>>,
        walls: Query<(Entity, Has<CreatingWall>), (With<Wall>, Without<Handle<Mesh>>)>,
    ) {
        for (entity, creating_wall) in &walls {
            debug!("initializing wall `{entity}`");

            let mut entity = commands.entity(entity);
            entity.insert((
                Apertures::default(),
                Collider::default(),
                CollisionLayers::new(Layer::Wall, Layer::Object),
                NoFrustumCulling,
                PbrBundle {
                    material: asset_server.load("base/walls/brick/brick.ron"),
                    mesh: meshes.add(DynamicMesh::create_empty()),
                    ..Default::default()
                },
            ));

            if !creating_wall {
                entity.insert(NavMeshAffector);
            }
        }
    }

    pub(crate) fn update_meshes(
        mut triangulator: Local<Triangulator>,
        mut meshes: ResMut<Assets<Mesh>>,
        mut changed_walls: Query<
            (
                &Handle<Mesh>,
                Ref<SplineSegment>,
                &SplineConnections,
                &mut Apertures,
                &mut Collider,
            ),
            Or<(Changed<SplineConnections>, Changed<Apertures>)>,
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

            // Creating walls shouldn't affect navigation.
            if apertures.collision_outdated || segment.is_changed() || collider.is_added() {
                trace!("regenerating wall collision");
                *collider = wall_mesh::generate_collider(*segment, &apertures);
                apertures.collision_outdated = false;
            }
        }
    }

    fn create(
        mut commands: Commands,
        mut create_events: EventReader<FromClient<WallCreate>>,
        mut confirm_events: EventWriter<ToClients<WallCreateConfirmed>>,
    ) {
        for FromClient { client_id, event } in create_events.read().copied() {
            info!("`{client_id:?}` spawns wall");
            // TODO: validate if wall can be spawned.
            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: WallCreateConfirmed,
            });
            commands.entity(event.lot_entity).with_children(|parent| {
                parent.spawn(WallBundle::new(event.segment));
            });
        }
    }
}

#[derive(Bundle)]
struct WallBundle {
    wall: Wall,
    spline_segment: SplineSegment,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl WallBundle {
    fn new(segment: SplineSegment) -> Self {
        Self {
            wall: Wall,
            spline_segment: segment,
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
    pub(super) collision_outdated: bool,
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

    /// Returns index of an aperture on the corresponding object entity.
    pub(crate) fn position(&self, entity: Entity) -> Option<usize> {
        self.iter()
            .position(|aperture| aperture.object_entity == entity)
    }

    /// Returns aperture by its index.
    pub(crate) fn remove(&mut self, index: usize) -> Aperture {
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

/// Client event to request a wall creation.
#[derive(Clone, Copy, Deserialize, Event, Serialize)]
struct WallCreate {
    lot_entity: Entity,
    segment: SplineSegment,
}

impl MapEntities for WallCreate {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        self.lot_entity = entity_mapper.map_entity(self.lot_entity);
    }
}

#[derive(Deserialize, Event, Serialize)]
struct WallCreateConfirmed;
