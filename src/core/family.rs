use std::{fs, iter};

use anyhow::{Context, Result};
use bevy::{
    ecs::{
        entity::{EntityMap, MapEntities, MapEntitiesError},
        reflect::ReflectMapEntities,
        system::{Command, EntityCommands},
    },
    prelude::*,
    reflect::TypeRegistry,
    scene::DynamicEntity,
};
use bevy_renet::renet::RenetServer;
use iyes_loopless::prelude::*;
use serde::{Deserialize, Serialize};

use super::{
    city::ActiveCity,
    doll::{ActiveDoll, DollSelect},
    error_message,
    game_paths::GamePaths,
    game_state::GameState,
    game_world::{save_rules::SaveRules, GameWorld},
    network::{
        network_event::client_event::{ClientEvent, ClientEventAppExt, ClientSendBuffer},
        replication::map_entity::ReflectMapEntity,
    },
};

#[derive(SystemLabel)]
pub(crate) enum FamilySystems {
    SaveSystem,
}

pub(super) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FamilySync>()
            .register_type::<Budget>()
            .add_mapped_client_event::<FamilyDespawn>()
            .add_event::<FamilySelect>()
            .add_event::<FamilySave>()
            .add_event::<FamilySaved>()
            .add_system(Self::family_sync_system.run_if_resource_exists::<GameWorld>())
            .add_system(
                Self::saving_system
                    .pipe(error_message::err_message_system)
                    .run_if_resource_exists::<GameWorld>()
                    .label(FamilySystems::SaveSystem),
            )
            .add_system(Self::selection_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::select_confirmation_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::deletion_system.run_if_resource_exists::<RenetServer>())
            .add_system(Self::state_enter_system.run_in_state(GameState::World))
            .add_system(Self::state_enter_system.run_in_state(GameState::FamilyEditor))
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>());
    }
}

impl FamilyPlugin {
    fn family_sync_system(
        mut commands: Commands,
        dolls: Query<(Entity, Option<&Family>, &FamilySync), Changed<FamilySync>>,
        mut families: Query<&mut Dolls>,
    ) {
        for (entity, family, family_sync) in &dolls {
            // Remove previous.
            if let Some(family) = family {
                if let Ok(mut dolls) = families.get_mut(family.0) {
                    let index = dolls
                        .iter()
                        .position(|&doll_entity| doll_entity == entity)
                        .expect("dolls should contain referenced entity");
                    dolls.swap_remove(index);
                }
            }

            commands.entity(entity).insert(Family(family_sync.0));
            if let Ok(mut dolls) = families.get_mut(family_sync.0) {
                dolls.push(entity);
            } else {
                commands.entity(family_sync.0).insert(Dolls(vec![entity]));
            }
        }
    }

    fn saving_system(
        mut save_events: EventReader<FamilySave>,
        mut set: ParamSet<(&World, EventWriter<FamilySaved>)>,
        save_rules: Res<SaveRules>,
        game_paths: Res<GamePaths>,
        registry: Res<AppTypeRegistry>,
        families: Query<(&Name, &Dolls)>,
    ) -> Result<()> {
        for entity in save_events.iter().map(|event| event.0) {
            let (name, dolls) = families
                .get(entity)
                .expect("family entity should contain dolls");

            let scene = save_to_scene(set.p0(), &registry, &save_rules, entity, dolls);
            let ron = scene
                .serialize_ron(&registry)
                .expect("game world should be serialized");

            fs::create_dir_all(&game_paths.families)
                .with_context(|| format!("unable to create {:?}", game_paths.families))?;

            let family_path = game_paths.family_path(name.as_str());
            fs::write(&family_path, ron)
                .with_context(|| format!("unable to save game to {family_path:?}"))?;

            set.p1().send_default();
        }

        Ok(())
    }

    /// Transforms [`FamilySelect`] events into [`DollSelect`] events with the first doll.
    fn selection_system(
        mut doll_select_buffer: ResMut<ClientSendBuffer<DollSelect>>,
        mut family_select_events: EventReader<FamilySelect>,
        families: Query<&Dolls>,
    ) {
        for event in family_select_events.iter() {
            let family = families
                .get(event.0)
                .expect("event entity should be a family");
            let doll_entity = *family
                .first()
                .expect("spawned family should always contain at least one doll");
            doll_select_buffer.push(DollSelect(doll_entity));
        }
    }

    fn select_confirmation_system(
        mut commands: Commands,
        active_dolls: Query<&Family, Added<ActiveDoll>>,
        active_families: Query<Entity, With<ActiveFamily>>,
    ) {
        for family in &active_dolls {
            if let Ok(previous_entity) = active_families.get_single() {
                commands.entity(previous_entity).remove::<ActiveFamily>();
            }
            commands.entity(family.0).insert(ActiveFamily);
        }
    }

    fn deletion_system(
        mut commands: Commands,
        mut despawn_events: EventReader<ClientEvent<FamilyDespawn>>,
    ) {
        for event in despawn_events.iter().map(|event| event.event) {
            commands.entity(event.0).despawn_family();
        }
    }

    fn state_enter_system(
        mut commands: Commands,
        parents: Query<&Parent>,
        active_families: Query<&Dolls, Added<ActiveFamily>>,
    ) {
        if let Ok(dolls) = active_families.get_single() {
            let doll_entity = *dolls
                .first()
                .expect("family should contain at least one doll");
            let parent = parents
                .get(doll_entity)
                .expect("doll should have a city as a parent");
            commands.entity(parent.get()).insert(ActiveCity);
            commands.insert_resource(NextState(GameState::Family));
        }
    }

    fn cleanup_system(mut commands: Commands, dolls: Query<Entity, With<Dolls>>) {
        for entity in &dolls {
            commands.entity(entity).despawn();
        }
    }
}

fn save_to_scene(
    world: &World,
    registry: &TypeRegistry,
    save_rules: &SaveRules,
    family_entity: Entity,
    dolls: &Dolls,
) -> DynamicScene {
    let mut scene = DynamicScene::default();
    scene.entities.reserve(dolls.len() + 1); // +1 because of `family_entity`.

    let registry = registry.read();
    for entity in dolls.iter().copied().chain(iter::once(family_entity)) {
        let mut dynamic_entity = DynamicEntity {
            entity: entity.index(),
            components: Vec::new(),
        };

        let entity = world.entity(entity);
        for component_id in entity.archetype().components().filter(|&component_id| {
            save_rules.persistent_component(entity.archetype(), component_id)
        }) {
            let component_info = unsafe { world.components().get_info_unchecked(component_id) };
            let type_name = component_info.name();
            let component = component_info
                .type_id()
                .and_then(|type_id| registry.get(type_id))
                .and_then(|registration| registration.data::<ReflectComponent>())
                .and_then(|reflect_component| reflect_component.reflect(world, entity.id()))
                .unwrap_or_else(|| panic!("non-ignored component {type_name} should be registered and have reflect(Component)"));
            dynamic_entity.components.push(component.clone_value());
        }

        scene.entities.push(dynamic_entity);
    }

    scene
}

#[derive(Bundle)]
pub(crate) struct FamilyBundle {
    name: Name,
    budget: Budget,
}

impl Default for FamilyBundle {
    fn default() -> Self {
        Self {
            name: Name::new("New family"),
            budget: Default::default(),
        }
    }
}

#[derive(Component)]
pub(crate) struct Family(pub(crate) Entity);

#[derive(Component, Default, Deref, DerefMut)]
pub(crate) struct Dolls(Vec<Entity>);

#[derive(Component, Reflect)]
#[reflect(Component, MapEntities, MapEntity)]
pub(crate) struct FamilySync(pub(crate) Entity);

// We need to impl either [`FromWorld`] or [`Default`] so [`FamilySync`] can be registered as [`Reflect`].
// Same technicue is used in Bevy for [`Parent`]
impl FromWorld for FamilySync {
    fn from_world(_world: &mut World) -> Self {
        Self(Entity::from_raw(u32::MAX))
    }
}

impl MapEntities for FamilySync {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

/// Indicates locally controlled family.
#[derive(Component)]
pub(crate) struct ActiveFamily;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Budget(u32);

pub(crate) struct FamilySave(pub(crate) Entity);

#[derive(Default)]
pub(crate) struct FamilySaved;

/// Selects a family entity to play using its first doll.
///
/// Its a local event that translates into a [`DollSelect`]
/// event with the first doll from selected family.
pub(crate) struct FamilySelect(pub(crate) Entity);

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct FamilyDespawn(pub(crate) Entity);

impl MapEntities for FamilyDespawn {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

pub(crate) trait DespawnFamilyExt {
    fn despawn_family(&mut self);
}

impl DespawnFamilyExt for EntityCommands<'_, '_, '_> {
    fn despawn_family(&mut self) {
        let family_entity = self.id();
        self.commands().add(DespawnFamily(family_entity));
    }
}

struct DespawnFamily(Entity);

impl Command for DespawnFamily {
    fn write(self, world: &mut World) {
        let mut family_entity = world.entity_mut(self.0);
        let dolls = family_entity
            .remove::<Dolls>()
            .expect("despawn family command should refer to a family entity");
        family_entity.despawn();
        for entity in dolls.0 {
            world.entity_mut(entity).despawn_recursive();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn saving() {
        let mut app = App::new();
        app.init_resource::<SaveRules>()
            .init_resource::<GamePaths>()
            .init_resource::<GameWorld>()
            .register_type::<Cow<'static, str>>() // To serialize Name, https://github.com/bevyengine/bevy/issues/5597
            .add_client_event::<DollSelect>()
            .add_plugins(MinimalPlugins)
            .add_plugin(FamilyPlugin);

        let doll_entity = app.world.spawn_empty().id();
        let family_entity = app
            .world
            .spawn((FamilyBundle::default(), Dolls(vec![doll_entity])))
            .id();

        app.world.send_event(FamilySave(family_entity));

        app.update();

        let saved_events = app.world.resource::<Events<FamilySaved>>();
        assert_eq!(saved_events.len(), 1);
    }
}
