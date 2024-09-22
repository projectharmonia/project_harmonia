pub(crate) mod door;
pub mod placing_object;
pub(crate) mod wall_mount;

use avian3d::prelude::*;
use bevy::{
    asset::AssetPath,
    ecs::{entity::MapEntities, reflect::ReflectCommandExt},
    prelude::*,
};
use bevy_mod_outline::OutlineBundle;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    city::{City, HALF_CITY_SIZE},
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
            .replicate_group::<(Object, Transform)>()
            .add_mapped_client_event::<CommandRequest<ObjectCommand>>(ChannelKind::Unordered)
            .add_systems(
                PreUpdate,
                Self::init
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PostUpdate,
                Self::apply_command
                    .before(ServerSet::StoreHierarchy)
                    .run_if(server_or_singleplayer),
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
                GlobalTransform::default(),
                VisibilityBundle::default(),
                CollisionLayers::new(
                    Layer::Object,
                    [Layer::PlacingObject, Layer::Wall, Layer::PlacingWall],
                ),
            ));

            for component in &info.components {
                entity.insert_reflect(component.clone_value());
            }
            for component in &info.spawn_components {
                entity.insert_reflect(component.clone_value());
            }
        }
    }

    fn apply_command(
        mut commands: Commands,
        mut request_events: EventReader<FromClient<CommandRequest<ObjectCommand>>>,
        mut confirm_events: EventWriter<ToClients<CommandConfirmation>>,
        mut objects: Query<&mut Transform, Without<City>>,
    ) {
        for FromClient { client_id, event } in request_events.read().cloned() {
            // TODO: validate if command can be applied.
            let mut confirmation = CommandConfirmation::new(event.id);
            match event.command {
                ObjectCommand::Buy {
                    info_path,
                    city_entity,
                    translation,
                    rotation,
                } => {
                    if translation.y.abs() > HALF_CITY_SIZE {
                        error!("received translation {translation} with 'y' outside of city size");
                        continue;
                    }

                    info!("`{client_id:?}` buys object {info_path:?}");
                    commands.entity(city_entity).with_children(|parent| {
                        let transform =
                            Transform::from_translation(translation).with_rotation(rotation);
                        let entity = parent.spawn(ObjectBundle::new(info_path, transform)).id();
                        confirmation.entity = Some(entity);
                    });
                }
                ObjectCommand::Move {
                    entity,
                    translation,
                    rotation,
                } => match objects.get_mut(entity) {
                    Ok(mut transform) => {
                        info!("`{client_id:?}` moves object `{entity}`");
                        transform.translation = translation;
                        transform.rotation = rotation;
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
    transform: Transform,
    parent_sync: ParentSync,
    replication: Replicated,
}

impl ObjectBundle {
    fn new(info_path: AssetPath<'static>, transform: Transform) -> Self {
        Self {
            object: Object(info_path),
            transform,
            parent_sync: Default::default(),
            replication: Replicated,
        }
    }
}

/// Contains path to the object info.
#[derive(Clone, Component, Debug, Default, Reflect, Serialize, Deserialize)]
#[reflect(Component)]
pub(crate) struct Object(AssetPath<'static>);

#[derive(Clone, Deserialize, Serialize)]
enum ObjectCommand {
    Buy {
        info_path: AssetPath<'static>,
        city_entity: Entity,
        translation: Vec3,
        rotation: Quat,
    },
    Move {
        entity: Entity,
        translation: Vec3,
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
        let reverse_command = match *self {
            Self::Buy { .. } => Self::Sell {
                // Correct entity will be set after the server confirmation.
                entity: Entity::PLACEHOLDER,
            },
            Self::Move { entity, .. } => {
                let transform = world.get::<Transform>(entity).unwrap();
                Self::Move {
                    entity,
                    translation: transform.translation,
                    rotation: transform.rotation,
                }
            }
            Self::Sell { entity } => {
                recorder.record(entity);
                let entity = world.entity(entity);
                let info_path = entity.get::<Object>().unwrap().0.clone();
                let parent = entity.get::<Parent>().unwrap();
                let transform = entity.get::<Transform>().unwrap();
                Self::Buy {
                    info_path,
                    city_entity: **parent,
                    translation: transform.translation,
                    rotation: transform.rotation,
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
        if let Self::Sell { entity } = &mut *self {
            *entity = confirmation
                .entity
                .expect("confirmation for object buying should contain an entity");
            recorder.record(*entity);
        }

        self
    }
}

impl MapEntities for ObjectCommand {
    fn map_entities<T: EntityMapper>(&mut self, entity_mapper: &mut T) {
        match self {
            Self::Buy { .. } => (),
            Self::Move { entity, .. } => *entity = entity_mapper.map_entity(*entity),
            Self::Sell { entity } => *entity = entity_mapper.map_entity(*entity),
        };
    }
}
