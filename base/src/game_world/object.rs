pub(crate) mod door;
pub mod placing_object;
pub(crate) mod wall_mount;

use avian3d::prelude::*;
use bevy::{
    asset::AssetPath,
    ecs::{entity::MapEntities, reflect::ReflectCommandExt},
    prelude::*,
};
use bevy_mod_outline::OutlineVolume;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    city::{City, HALF_CITY_SIZE},
    commands_history::{
        CommandConfirmation, CommandId, CommandRequest, ConfirmableCommand, EntityRecorder,
        PendingCommand,
    },
    highlighting::HIGHLIGHTING_VOLUME,
};
use crate::{asset::manifest::object_manifest::ObjectManifest, game_world::Layer};
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
            .add_observer(Self::init)
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
        trigger: Trigger<OnAdd, Object>,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        manifests: Res<Assets<ObjectManifest>>,
        mut objects: Query<(&Object, &mut Name, &mut SceneRoot)>,
    ) {
        let (object, mut name, mut scene_root) = objects.get_mut(trigger.entity()).unwrap();
        let Some(manifest_handle) = asset_server.get_handle(&**object) else {
            error!("'{}' is missing, ignoring", &**object);
            return;
        };

        debug!(
            "initializing object '{}' for `{}`",
            &**object,
            trigger.entity()
        );

        let manifest = manifests
            .get(&manifest_handle)
            .unwrap_or_else(|| panic!("'{:?}' should be loaded", &**object));

        *name = Name::new(manifest.general.name.clone());
        scene_root.0 = asset_server.load(manifest.scene.clone());

        let mut entity = commands.entity(trigger.entity());
        for component in &manifest.components {
            entity.insert_reflect(component.clone_value());
        }
        for component in &manifest.spawn_components {
            entity.insert_reflect(component.clone_value());
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
                    manifest_path,
                    city_entity,
                    translation,
                    rotation,
                } => {
                    if translation.y.abs() > HALF_CITY_SIZE {
                        error!("received translation {translation} with 'y' outside of city size");
                        continue;
                    }

                    info!("`{client_id:?}` buys object {manifest_path:?}");
                    commands.entity(city_entity).with_children(|parent| {
                        let transform =
                            Transform::from_translation(translation).with_rotation(rotation);
                        let entity = parent.spawn((Object(manifest_path), transform)).id();
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

/// Contains path to the object info.
#[derive(Clone, Component, Debug, Default, Reflect, Serialize, Deserialize, Deref)]
#[reflect(Component)]
#[require(
    ParentSync,
    Replicated,
    SceneRoot,
    Name,
    RigidBody(|| RigidBody::Kinematic),
    OutlineVolume(|| HIGHLIGHTING_VOLUME),
    CollisionLayers(|| CollisionLayers::new(
        Layer::Object,
        [Layer::PlacingObject, Layer::Wall, Layer::PlacingWall],
    ))
)]
pub(crate) struct Object(pub(crate) AssetPath<'static>);

#[derive(Clone, Deserialize, Serialize)]
enum ObjectCommand {
    Buy {
        manifest_path: AssetPath<'static>,
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
                let manifest_path = entity.get::<Object>().unwrap().0.clone();
                let parent = entity.get::<Parent>().unwrap();
                let transform = entity.get::<Transform>().unwrap();
                Self::Buy {
                    manifest_path,
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
