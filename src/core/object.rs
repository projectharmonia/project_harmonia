pub(crate) mod cursor_object;

use std::path::PathBuf;

use bevy::{
    app::PluginGroupBuilder,
    ecs::entity::{EntityMap, MapEntities, MapEntitiesError},
    prelude::*,
};
use bevy_mod_outline::{Outline, OutlineBundle};
use bevy_mod_raycast::{RayCastMesh, RayCastSource};
use bevy_renet::renet::RenetClient;
use bevy_scene_hook::SceneHook;
use derive_more::From;
use iyes_loopless::prelude::*;
use leafwing_input_manager::prelude::ActionState;
use serde::{Deserialize, Serialize};

use super::{
    asset_metadata,
    control_action::ControlAction,
    game_state::GameState,
    game_world::{parent_sync::ParentSync, GameEntity},
    network::{
        entity_serde,
        network_event::client_event::{
            ClientEvent, ClientEventAppExt, ClientEventSystems, ClientSendBuffer,
        },
        server::SERVER_ID,
    },
};
use cursor_object::{CursorObject, CursorObjectPlugin};

pub(super) struct ObjectPlugins;

impl PluginGroup for ObjectPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(CursorObjectPlugin).add(ObjectPlugin);
    }
}

struct ObjectPlugin;

impl Plugin for ObjectPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Picked>()
            .register_type::<ObjectPath>()
            .add_mapped_client_event::<ObjectPick>()
            .add_client_event::<PickCancel>()
            .add_client_event::<PickDelete>()
            .add_system(Self::spawning_system.run_in_state(GameState::City))
            .add_system(
                Self::ray_system
                    .chain(Self::picking_system)
                    .chain(Self::outline_system)
                    .run_in_state(GameState::City)
                    .run_if_not(cursor_object::is_cursor_object_exists)
                    .before(ClientEventSystems::<ObjectPick>::MappingSystem),
            )
            .add_system(Self::pick_confirmation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::pick_cancellation_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::pick_deletion_system.run_unless_resource_exists::<RenetClient>())
            .add_system(Self::cursor_spawning_system.run_in_state(GameState::City));
    }
}

impl ObjectPlugin {
    fn spawning_system(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        spawned_objects: Query<(Entity, &ObjectPath), Added<ObjectPath>>,
    ) {
        for (entity, object_path) in &spawned_objects {
            let scene_path = asset_metadata::scene_path(&object_path.0);
            let scene_handle: Handle<Scene> = asset_server.load(&scene_path);

            commands
                .entity(entity)
                .insert(scene_handle)
                .insert(GlobalTransform::default())
                .insert(SceneHook::new(|entity, commands| {
                    if entity.contains::<Handle<Mesh>>() {
                        commands
                            .insert_bundle(OutlineBundle {
                                outline: Outline {
                                    visible: false,
                                    colour: Color::rgba(1.0, 1.0, 1.0, 0.3),
                                    width: 2.0,
                                },
                                ..Default::default()
                            })
                            .insert(RayCastMesh::<ObjectPath>::default());
                    }
                }))
                .insert_bundle(VisibilityBundle::default());
            debug!("spawned object {scene_path:?}");
        }
    }

    fn ray_system(
        ray_sources: Query<&RayCastSource<ObjectPath>>,
        parents: Query<(&Parent, Option<&ObjectPath>)>,
    ) -> Option<Entity> {
        for source in &ray_sources {
            if let Some((child_entity, _)) = source.intersect_top() {
                let entity = find_parent_object(child_entity, &parents)
                    .expect("object entity should have a parent");
                return Some(entity);
            }
        }

        None
    }

    fn picking_system(
        In(entity): In<Option<Entity>>,
        mut pick_buffer: ResMut<ClientSendBuffer<ObjectPick>>,
        action_state: Res<ActionState<ControlAction>>,
    ) -> Option<Entity> {
        if let Some(entity) = entity {
            if action_state.just_pressed(ControlAction::Confirm) {
                pick_buffer.push(ObjectPick(entity));
                None
            } else {
                Some(entity)
            }
        } else {
            None
        }
    }

    fn outline_system(
        In(entity): In<Option<Entity>>,
        mut previous_entity: Local<Option<Entity>>,
        mut outlines: Query<&mut Outline>,
        children: Query<&Children>,
    ) {
        if *previous_entity == entity {
            return;
        }

        if let Some(entity) = entity {
            set_outline_recursive(entity, true, &mut outlines, &children);
        }

        if let Some(entity) = *previous_entity {
            set_outline_recursive(entity, false, &mut outlines, &children);
        }

        *previous_entity = entity;
    }

    fn pick_confirmation_system(
        mut commands: Commands,
        mut pick_events: EventReader<ClientEvent<ObjectPick>>,
        unpicked_objects: Query<(), (With<ObjectPath>, Without<Picked>)>,
    ) {
        for ClientEvent { client_id, event } in pick_events.iter().copied() {
            if unpicked_objects.get(event.0).is_ok() {
                commands.entity(event.0).insert(Picked(client_id));
            }
        }
    }

    fn pick_cancellation_system(
        mut commands: Commands,
        mut cancel_events: EventReader<ClientEvent<PickCancel>>,
        picked_objects: Query<(Entity, &Picked)>,
    ) {
        for ClientEvent { client_id, .. } in cancel_events.iter().copied() {
            for (entity, picked) in &picked_objects {
                if picked.0 == client_id {
                    commands.entity(entity).remove::<Picked>();
                }
            }
        }
    }

    fn pick_deletion_system(
        mut commands: Commands,
        mut delete_events: EventReader<ClientEvent<PickDelete>>,
        picked_objects: Query<(Entity, &Picked)>,
    ) {
        for ClientEvent { client_id, .. } in delete_events.iter().copied() {
            for (entity, picked) in &picked_objects {
                if picked.0 == client_id {
                    commands.entity(entity).despawn_recursive();
                }
            }
        }
    }

    fn cursor_spawning_system(
        mut commands: Commands,
        client: Option<Res<RenetClient>>,
        mut picked_objects: Query<(Entity, &Parent, &Picked), Added<Picked>>,
    ) {
        let client_id = client.map(|client| client.client_id()).unwrap_or(SERVER_ID);
        for (entity, parent, picked) in &mut picked_objects {
            if picked.0 == client_id {
                commands.entity(parent.get()).with_children(|parent| {
                    parent.spawn().insert(CursorObject::Moving(entity));
                });
            }
        }
    }
}

/// Iterates up the hierarchy until it finds a parent with an [`ObjectPath`] component if exists.
fn find_parent_object(
    entity: Entity,
    parents: &Query<(&Parent, Option<&ObjectPath>)>,
) -> Option<Entity> {
    let (parent, object_path) = parents.get(entity).unwrap();
    if object_path.is_some() {
        return Some(entity);
    }

    find_parent_object(parent.get(), parents)
}

fn set_outline_recursive(
    entity: Entity,
    visible: bool,
    outlines: &mut Query<&mut Outline>,
    children: &Query<&Children>,
) {
    if let Ok(mut outline) = outlines.get_mut(entity) {
        outline.visible = visible;
    }

    if let Ok(entity_children) = children.get(entity) {
        for &entity in entity_children {
            set_outline_recursive(entity, visible, outlines, children);
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct ObjectPick(#[serde(with = "entity_serde")] pub(super) Entity);

impl MapEntities for ObjectPick {
    #[cfg_attr(coverage, no_coverage)]
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct PickCancel;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
struct PickDelete;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(super) struct Picked(u64);

#[derive(Bundle)]
pub(crate) struct ObjectBundle {
    object_path: ObjectPath,
    transform: Transform,
    parent_sync: ParentSync,
    game_entity: GameEntity,
}

impl ObjectBundle {
    fn new(metadata_path: PathBuf, translation: Vec3, rotation: Quat, parent: Entity) -> Self {
        Self {
            object_path: ObjectPath(
                metadata_path
                    .into_os_string()
                    .into_string()
                    .expect("Path should be a UTF-8 string"),
            ),
            transform: Transform::default()
                .with_translation(translation)
                .with_rotation(rotation),
            parent_sync: ParentSync(parent),
            game_entity: GameEntity,
        }
    }
}

/// Contains path to an object metadata file.
// TODO Bevy 0.9: Use `PathBuf`: https://github.com/bevyengine/bevy/issues/6166
#[derive(Clone, Component, Debug, Default, From, Reflect)]
#[reflect(Component)]
pub(crate) struct ObjectPath(String);

#[cfg(test)]
mod tests {
    use bevy::{
        asset::AssetPlugin, core::CorePlugin, ecs::system::SystemState, scene::ScenePlugin,
    };
    use bevy_mod_raycast::IntersectionData;
    use bevy_scene_hook::HookPlugin;
    use itertools::Itertools;
    use shape::Cube;

    use super::*;

    #[test]
    fn parent_search() {
        let mut world = World::new();
        let child_entity = world.spawn().id();
        let parent_entity = world
            .spawn()
            .insert(ObjectPath::default())
            .push_children(&[child_entity])
            .id();

        // Assign a parent, as an outline object is always expected to have a parent object.
        world.spawn().push_children(&[parent_entity]);

        let mut system_state: SystemState<Query<(&Parent, Option<&ObjectPath>)>> =
            SystemState::new(&mut world);

        let entity = find_parent_object(child_entity, &system_state.get(&world))
            .expect("object should have a parent");
        assert_eq!(entity, parent_entity);
    }

    #[test]
    fn recursive_outline() {
        let mut world = World::new();
        let child_entity1 = world.spawn().insert(Outline::default()).id();
        let child_entity2 = world
            .spawn()
            .insert(Outline::default())
            .push_children(&[child_entity1])
            .id();
        let root_entity = world
            .spawn()
            .insert(Outline::default())
            .push_children(&[child_entity2])
            .id();

        let mut system_state: SystemState<(Query<&mut Outline>, Query<&Children>)> =
            SystemState::new(&mut world);

        const VISIBLE: bool = false;
        let (mut outlines, children) = system_state.get_mut(&mut world);
        set_outline_recursive(root_entity, VISIBLE, &mut outlines, &children);

        assert_eq!(
            world.get::<Outline>(child_entity1).unwrap().visible,
            VISIBLE
        );
        assert_eq!(
            world.get::<Outline>(child_entity2).unwrap().visible,
            VISIBLE
        );
        assert_eq!(world.get::<Outline>(root_entity).unwrap().visible, VISIBLE);
    }

    #[test]
    fn spawning() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin)
            .add_asset::<Mesh>()
            .add_plugin(ScenePlugin)
            .add_plugin(HookPlugin);

        let object_entity = app.world.spawn().insert(ObjectPath::default()).id();

        app.update();

        // Manually create a scene with a mesh to trigger hook
        let mut world = World::new();
        let mut meshes = app.world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes.add(Cube::default().into());
        world.spawn().insert(mesh_handle);
        let mut scenes = app.world.resource_mut::<Assets<Scene>>();
        let scene_handle = scenes.add(Scene::new(world));

        let mut object_entity = app.world.entity_mut(object_entity);
        object_entity.insert(scene_handle);

        assert!(object_entity.contains::<Handle<Scene>>());
        assert!(object_entity.contains::<GlobalTransform>());
        assert!(object_entity.contains::<Visibility>());
        assert!(object_entity.contains::<ComputedVisibility>());

        let object_entity = object_entity.id();

        app.update();

        let parent = app
            .world
            .query_filtered::<&Parent, With<Outline>>()
            .single(&app.world);
        assert_eq!(parent.get(), object_entity);
    }

    #[test]
    fn hovering() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let outline_entity = app
            .world
            .spawn()
            .insert(Outline::default())
            .insert(ObjectPath::default())
            .id();
        app.world.spawn().push_children(&[outline_entity]);

        let mut ray_source = RayCastSource::<ObjectPath>::default();
        ray_source.intersections_mut().push((
            outline_entity,
            IntersectionData::new(Vec3::default(), Vec3::default(), 0.0, None),
        ));
        let ray_entity = app.world.spawn().insert(ray_source).id();

        app.update();

        assert!(app.world.get::<Outline>(outline_entity).unwrap().visible);

        let next_outline_entity = app
            .world
            .spawn()
            .insert(Outline::default())
            .insert(ObjectPath::default())
            .id();
        app.world.spawn().push_children(&[next_outline_entity]);
        let mut ray_source = app
            .world
            .get_mut::<RayCastSource<ObjectPath>>(ray_entity)
            .unwrap();
        ray_source.intersections_mut().clear();
        ray_source.intersections_mut().push((
            next_outline_entity,
            IntersectionData::new(Vec3::default(), Vec3::default(), 0.0, None),
        ));

        app.update();

        assert!(!app.world.get::<Outline>(outline_entity).unwrap().visible);
        assert!(
            app.world
                .get::<Outline>(next_outline_entity)
                .unwrap()
                .visible
        );
    }

    #[test]
    fn no_hovering() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let outline_entity = app
            .world
            .spawn()
            .insert(Outline::default())
            .insert(ObjectPath::default())
            .id();

        app.world
            .spawn()
            .insert(RayCastSource::<ObjectPath>::default());

        app.update();

        let outline = app.world.get::<Outline>(outline_entity).unwrap();
        assert!(!outline.visible);
    }

    #[test]
    fn picking() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let outline_entity = app.world.spawn().insert(ObjectPath::default()).id();
        app.world.spawn().push_children(&[outline_entity]);

        let mut ray_source = RayCastSource::<ObjectPath>::default();
        ray_source.intersections_mut().push((
            outline_entity,
            IntersectionData::new(Vec3::default(), Vec3::default(), 0.0, None),
        ));
        app.world.spawn().insert(ray_source);

        app.world
            .resource_mut::<ActionState<ControlAction>>()
            .press(ControlAction::Confirm);

        app.update();

        let pick_buffer = app.world.resource::<ClientSendBuffer<ObjectPick>>();
        let pick_event = pick_buffer.iter().exactly_one().unwrap();
        assert_eq!(pick_event.0, outline_entity);
    }

    #[test]
    fn pick_confirmation() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        const CLIENT_ID: u64 = 1;
        let object_entity = app.world.spawn().insert(ObjectPath::default()).id();
        let mut pick_events = app.world.resource_mut::<Events<ClientEvent<ObjectPick>>>();
        pick_events.send(ClientEvent {
            client_id: CLIENT_ID,
            event: ObjectPick(object_entity),
        });

        app.update();

        assert_eq!(app.world.get::<Picked>(object_entity).unwrap().0, CLIENT_ID);
    }

    #[test]
    fn pick_cancellation() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        const CLIENT_ID: u64 = 1;
        let object_entity = app.world.spawn().insert(Picked(CLIENT_ID)).id();
        let mut pick_events = app.world.resource_mut::<Events<ClientEvent<PickCancel>>>();
        pick_events.send(ClientEvent {
            client_id: CLIENT_ID,
            event: PickCancel,
        });

        app.update();

        assert!(app.world.get::<Picked>(object_entity).is_none());
    }

    #[test]
    fn pick_deletion() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        const CLIENT_ID: u64 = 1;
        let object_entity = app.world.spawn().insert(Picked(CLIENT_ID)).id();
        let mut pick_events = app.world.resource_mut::<Events<ClientEvent<PickDelete>>>();
        pick_events.send(ClientEvent {
            client_id: CLIENT_ID,
            event: PickDelete,
        });

        app.update();

        assert!(app.world.get_entity(object_entity).is_none());
    }

    #[test]
    fn cursor_spawning() {
        let mut app = App::new();
        app.add_plugin(TestMovingObjectPlugin);

        let object_entity = app.world.spawn().insert(Picked(SERVER_ID)).id();
        let parent_entity = app.world.spawn().push_children(&[object_entity]).id();

        app.update();

        let (parent, cursor_object) = app
            .world
            .query::<(&Parent, &CursorObject)>()
            .single(&app.world);

        assert_eq!(parent.get(), parent_entity);
        assert!(matches!(cursor_object, CursorObject::Moving(entity) if *entity == object_entity));
    }

    struct TestMovingObjectPlugin;

    impl Plugin for TestMovingObjectPlugin {
        fn build(&self, app: &mut App) {
            app.add_loopless_state(GameState::City)
                .init_resource::<ActionState<ControlAction>>()
                .add_plugin(CorePlugin)
                .add_plugin(AssetPlugin)
                .add_plugin(ObjectPlugin);
        }
    }
}
