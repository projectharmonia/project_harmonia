pub(crate) mod door;
pub mod placing_object;
pub(crate) mod wall_mount;

use bevy::{
    asset::AssetPath,
    ecs::{entity::MapEntities, reflect::ReflectCommandExt},
    prelude::*,
    render::{mesh::Indices, render_resource::PrimitiveTopology},
    scene::{self, SceneInstanceReady},
};
use bevy_mod_outline::{InheritOutlineBundle, OutlineBundle};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    city::{lot::LotVertices, City, HALF_CITY_SIZE},
    commands_history::{
        CommandConfirmation, CommandId, CommandRequest, ConfirmableCommand, EntityRecorder,
        PendingCommand,
    },
    hover::{highlighting::OutlineHighlightingExt, Hoverable},
};
use crate::{asset::info::object_info::ObjectInfo, core::GameState, game_world::Layer};
use door::DoorPlugin;
use placing_object::PlacingObjectPlugin;
use wall_mount::WallMountPlugin;

pub(super) struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((DoorPlugin, PlacingObjectPlugin, WallMountPlugin))
            .register_type::<Object>()
            .replicate::<Object>()
            .add_mapped_client_event::<CommandRequest<ObjectCommand>>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .run_if(in_state(GameState::InGame))
                    .after(ClientSet::Receive),
            )
            .add_systems(
                SpawnScene,
                Self::init_children
                    .run_if(in_state(GameState::InGame))
                    .after(scene::scene_spawner_system),
            )
            .add_systems(
                PostUpdate,
                Self::apply_command
                    .before(ServerSet::StoreHierarchy)
                    .run_if(has_authority),
            );
    }
}

impl ObjectPlugin {
    fn init(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        objects_info: Res<Assets<ObjectInfo>>,
        spawned_objects: Query<(Entity, &Object), Without<Handle<Scene>>>,
    ) {
        for (entity, object) in &spawned_objects {
            let info_handle = asset_server
                .get_handle(&object.0)
                .expect("info should be preloaded");
            let info = objects_info.get(&info_handle).unwrap();

            debug!("initializing object '{}' for `{entity}`", object.0);

            let scene_handle: Handle<Scene> = asset_server.load(info.scene.clone());
            let mut entity = commands.entity(entity);
            entity.insert((
                scene_handle,
                Name::new(info.general.name.clone()),
                Hoverable,
                RigidBody::Kinematic,
                OutlineBundle::highlighting(),
                SpatialBundle::default(),
                CollisionLayers::new(Layer::Object, [Layer::Object, Layer::Wall]),
            ));

            for component in &info.components {
                entity.insert_reflect(component.clone_value());
            }
            for component in &info.spawn_components {
                entity.insert_reflect(component.clone_value());
            }
        }
    }

    fn init_children(
        mut commands: Commands,
        mut ready_events: EventReader<SceneInstanceReady>,
        meshes: Res<Assets<Mesh>>,
        objects: Query<Entity, With<Object>>,
        chidlren: Query<&Children>,
        child_meshes: Query<(&Transform, &Handle<Mesh>)>,
    ) {
        for object_entity in objects.iter_many(ready_events.read().map(|event| event.parent)) {
            let mut merged_mesh = Mesh::new(PrimitiveTopology::TriangleList, Default::default())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<Vec3>::new())
                .with_inserted_indices(Indices::U32(Vec::new()));

            for child_entity in chidlren.iter_descendants(object_entity) {
                commands
                    .entity(child_entity)
                    .insert(InheritOutlineBundle::default());

                if let Ok((&transform, mesh_handle)) = child_meshes.get(child_entity) {
                    let mut mesh = meshes
                        .get(mesh_handle)
                        .cloned()
                        .expect("scene mesh should always be valid");
                    mesh.transform_by(transform);
                    merged_mesh.merge(&mesh);
                }
            }

            let collider = Collider::convex_hull_from_mesh(&merged_mesh)
                .expect("object mesh should be in compatible format");

            debug!("inserting collider for `{object_entity}`");
            commands.entity(object_entity).insert(collider);
        }
    }

    fn apply_command(
        mut commands: Commands,
        mut request_events: EventReader<FromClient<CommandRequest<ObjectCommand>>>,
        mut confirm_events: EventWriter<ToClients<CommandConfirmation>>,
        mut objects: Query<(&mut Position, &mut Rotation)>,
        cities: Query<(Entity, &Transform), With<City>>,
        lots: Query<(Entity, &LotVertices)>,
    ) {
        for FromClient { client_id, event } in request_events.read().cloned() {
            let mut confirmation = CommandConfirmation::new(event.id);
            match event.command {
                ObjectCommand::Buy {
                    info_path,
                    position,
                    rotation,
                } => {
                    if position.y.abs() > HALF_CITY_SIZE {
                        error!("received position {position} with 'y' outside of city size");
                        continue;
                    }

                    let Some((city_entity, _)) = cities
                        .iter()
                        .map(|(entity, transform)| (entity, transform.translation.x - position.x))
                        .find(|(_, x)| x.abs() < HALF_CITY_SIZE)
                    else {
                        error!("unable to find a city for position {position}");
                        continue;
                    };

                    // TODO: Add a check if user can spawn an object on the lot.
                    let parent_entity = lots
                        .iter()
                        .find(|(_, vertices)| vertices.contains_point(position.xz()))
                        .map(|(lot_entity, _)| lot_entity)
                        .unwrap_or(city_entity);

                    info!("`{client_id:?}` buys object {info_path:?}");
                    commands.entity(parent_entity).with_children(|parent| {
                        let entity = parent
                            .spawn(ObjectBundle::new(info_path, position, rotation))
                            .id();
                        confirmation.entity = Some(entity);
                    });
                }
                ObjectCommand::Move {
                    entity,
                    position,
                    rotation,
                } => match objects.get_mut(entity) {
                    Ok((mut object_position, mut object_rotation)) => {
                        info!("`{client_id:?}` moves object `{entity}`");
                        **object_position = position;
                        **object_rotation = rotation;
                    }
                    Err(e) => error!("unable to move object `{entity}`: {e}"),
                },
                ObjectCommand::Sell { entity } => {
                    info!("`{client_id:?}` sells object `{entity}`");
                    commands.entity(entity).despawn_recursive();
                }
            }

            confirm_events.send(ToClients {
                mode: SendMode::Direct(client_id),
                event: confirmation,
            });
        }
    }
}

#[derive(Bundle)]
struct ObjectBundle {
    object: Object,
    position: Position,
    rotation: Rotation,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl ObjectBundle {
    fn new(info_path: AssetPath<'static>, translation: Vec3, rotation: Quat) -> Self {
        Self {
            object: Object(info_path),
            position: Position(translation),
            rotation: Rotation(rotation),
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

/// Contains path to the object info.
#[derive(Clone, Component, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(crate) struct Object(AssetPath<'static>);

#[derive(Clone, Debug, Deserialize, Serialize)]
enum ObjectCommand {
    Buy {
        info_path: AssetPath<'static>,
        position: Vec3,
        rotation: Quat,
    },
    Move {
        entity: Entity,
        position: Vec3,
        rotation: Quat,
    },
    Sell {
        entity: Entity,
    },
}

impl PendingCommand for ObjectCommand {
    fn apply(
        self: Box<Self>,
        id: CommandId,
        mut recorder: EntityRecorder,
        world: &mut World,
    ) -> Box<dyn ConfirmableCommand> {
        let reverse_command = match &*self {
            Self::Buy { .. } => Self::Sell {
                // Correct entity will be set after the server confirmation.
                entity: Entity::PLACEHOLDER,
            },
            Self::Move { entity, .. } => {
                let entity = world.entity(*entity);
                let position = **entity.get::<Position>().unwrap();
                let rotation = **entity.get::<Rotation>().unwrap();
                Self::Move {
                    entity: entity.id(),
                    position,
                    rotation,
                }
            }
            Self::Sell { entity } => {
                recorder.record(*entity);
                let entity = world.entity(*entity);
                let info_path = entity.get::<Object>().unwrap().0.clone();
                let position = **entity.get::<Position>().unwrap();
                let rotation = **entity.get::<Rotation>().unwrap();
                Self::Buy {
                    info_path,
                    position,
                    rotation,
                }
            }
        };

        world.send_event(CommandRequest { id, command: *self });

        Box::new(reverse_command)
    }
}

impl ConfirmableCommand for ObjectCommand {
    fn confirm(
        mut self: Box<Self>,
        mut recorder: EntityRecorder,
        confirmation: CommandConfirmation,
    ) -> Box<dyn PendingCommand> {
        if let ObjectCommand::Sell { entity } = &mut *self {
            *entity = confirmation
                .entity
                .expect("confirmation for buying should contain an entity");
            recorder.record(*entity);
        }

        self
    }
}

impl MapEntities for ObjectCommand {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        match self {
            ObjectCommand::Buy { .. } => (),
            ObjectCommand::Move { entity, .. } => *entity = entity_mapper.map_entity(*entity),
            ObjectCommand::Sell { entity } => *entity = entity_mapper.map_entity(*entity),
        };
    }
}
