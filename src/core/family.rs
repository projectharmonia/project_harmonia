use std::{fs, iter};

use anyhow::{Context, Result};
use bevy::{prelude::*, reflect::TypeRegistry, scene::DynamicEntity};
use iyes_loopless::prelude::*;

use super::{
    error_message,
    game_paths::GamePaths,
    game_state::GameState,
    game_world::{ignore_rules::IgnoreRules, GameWorld},
};

#[derive(SystemLabel)]
pub(crate) enum FamilySystems {
    SaveSystem,
}

pub(super) struct FamilyPlugin;

impl Plugin for FamilyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Family>()
            .register_type::<Budget>()
            .add_event::<FamilySave>()
            .add_event::<FamilySaved>()
            .add_system(
                Self::saving_system
                    .chain(error_message::err_message_system)
                    .run_if_resource_exists::<GameWorld>()
                    .label(FamilySystems::SaveSystem),
            );
    }
}

impl FamilyPlugin {
    fn saving_system(
        mut commands: Commands,
        mut save_events: EventReader<FamilySave>,
        mut set: ParamSet<(&World, EventWriter<FamilySaved>)>,
        ignore_rules: Res<IgnoreRules>,
        game_paths: Res<GamePaths>,
        registry: Res<TypeRegistry>,
        families: Query<(&Name, &Family)>,
    ) -> Result<()> {
        for entity in save_events.iter().map(|event| event.0) {
            let (name, family) = families
                .get(entity)
                .expect("family entity should contain family members");

            let scene = save_to_scene(set.p0(), &registry, &ignore_rules, entity, family);
            let ron = scene
                .serialize_ron(&registry)
                .expect("game world should be serialized");

            fs::create_dir_all(&game_paths.families)
                .with_context(|| format!("unable to create {:?}", game_paths.families))?;

            let family_path = game_paths.family_path(name.as_str());
            fs::write(&family_path, ron)
                .with_context(|| format!("unable to save game to {family_path:?}"))?;

            set.p1().send_default();
            commands.insert_resource(NextState(GameState::World));
        }

        Ok(())
    }
}

fn save_to_scene(
    world: &World,
    registry: &TypeRegistry,
    ignore_rules: &IgnoreRules,
    family_entity: Entity,
    family: &Family,
) -> DynamicScene {
    let mut scene = DynamicScene::default();
    scene.entities.reserve(family.len() + 1); // +1 because of `family_entity`.

    let registry = registry.read();
    for entity in family.iter().copied().chain(iter::once(family_entity)) {
        let mut dynamic_entity = DynamicEntity {
            entity: entity.id(),
            components: Vec::new(),
        };

        let entity = world.entity(entity);
        for component_id in entity.archetype().components().filter(|&component_id| {
            !ignore_rules.ignored_component(entity.archetype(), component_id)
        }) {
            let component_info = unsafe { world.components().get_info_unchecked(component_id) };
            let type_name = component_info.name();
            let reflect = component_info
                .type_id()
                .and_then(|type_id| registry.get(type_id))
                .and_then(|registration| registration.data::<ReflectComponent>())
                .and_then(|reflect_component| reflect_component.reflect(world, entity.id()))
                .unwrap_or_else(|| panic!("non-ignored component {type_name} should be reflected"));
            dynamic_entity.components.push(reflect.clone_value());
        }

        scene.entities.push(dynamic_entity);
    }

    scene
}

#[derive(Bundle)]
pub(crate) struct FamilyBundle {
    name: Name,
    family: Family,
    budget: Budget,
}

impl Default for FamilyBundle {
    fn default() -> Self {
        Self {
            name: Name::new("New family"),
            family: Default::default(),
            budget: Default::default(),
        }
    }
}

#[derive(Component, Default, Deref, DerefMut, Reflect)]
#[reflect(Component)]
pub(crate) struct Family(Vec<Entity>);

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Budget(u32);

pub(crate) struct FamilySave(pub(crate) Entity);

#[derive(Default)]
pub(crate) struct FamilySaved;

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;

    #[test]
    fn saving() {
        let mut app = App::new();
        app.init_resource::<IgnoreRules>()
            .init_resource::<GamePaths>()
            .init_resource::<GameWorld>()
            .register_type::<Cow<'static, str>>() // To serialize Name, https://github.com/bevyengine/bevy/issues/5597
            .add_plugins(MinimalPlugins)
            .add_plugin(FamilyPlugin);

        let family_entity = app
            .world
            .spawn()
            .insert_bundle(FamilyBundle::default())
            .id();

        let mut save_events = app.world.resource_mut::<Events<FamilySave>>();
        save_events.send(FamilySave(family_entity));

        app.update();

        let saved_events = app.world.resource::<Events<FamilySaved>>();
        assert_eq!(saved_events.len(), 1);
    }
}
