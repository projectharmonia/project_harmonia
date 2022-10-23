use bevy::prelude::*;
use derive_more::Display;
use iyes_loopless::prelude::IntoConditionalSystem;

use super::game_world::GameWorld;

pub(super) struct DollPlugin;

impl Plugin for DollPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FirstName>()
            .register_type::<LastName>()
            .add_system(Self::name_update_system.run_if_resource_exists::<GameWorld>())
            .add_system(Self::mesh_add_system.run_if_resource_exists::<GameWorld>());
    }
}

impl DollPlugin {
    fn name_update_system(
        mut commands: Commands,
        mut changed_names: Query<
            (Entity, &FirstName, &LastName),
            Or<(Changed<FirstName>, Changed<LastName>)>,
        >,
    ) {
        for (entity, first_name, last_name) in &mut changed_names {
            commands
                .entity(entity)
                .insert(Name::new(format!("{first_name} {last_name}")));
        }
    }

    fn mesh_add_system(
        mut commands: Commands,
        mut meshes: ResMut<Assets<Mesh>>,
        mut materials: ResMut<Assets<StandardMaterial>>,
        new_dolls: Query<Entity, Added<FirstName>>,
    ) {
        for entity in &new_dolls {
            commands
                .entity(entity)
                .insert_bundle(VisibilityBundle::default())
                .insert(GlobalTransform::default())
                .insert(meshes.add(Mesh::from(shape::Capsule::default())))
                .insert(materials.add(Color::rgb(0.3, 0.3, 0.3).into()));
        }
    }
}

#[derive(Component, Default, Deref, DerefMut, Display, Reflect)]
#[reflect(Component)]
pub(crate) struct FirstName(pub(crate) String);

#[derive(Component, Default, Deref, DerefMut, Display, Reflect)]
#[reflect(Component)]
pub(crate) struct LastName(pub(crate) String);

#[derive(Bundle, Default)]
pub(crate) struct DollBundle {
    first_name: FirstName,
    last_name: LastName,
    transform: Transform,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tests::HeadlessRenderPlugin;

    #[test]
    fn name_update() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_plugin(HeadlessRenderPlugin)
            .add_plugin(DollPlugin);

        const FIRST_NAME: &str = "First";
        const LAST_NAME: &str = "Last";
        let named_entity = app
            .world
            .spawn()
            .insert(FirstName(FIRST_NAME.to_string()))
            .insert(LastName(LAST_NAME.to_string()))
            .id();

        app.update();

        let name = app.world.get::<Name>(named_entity).unwrap();
        assert_eq!(name.as_str(), format!("{FIRST_NAME} {LAST_NAME}"));
    }

    #[test]
    fn mesh_add() {
        let mut app = App::new();
        app.init_resource::<GameWorld>()
            .add_plugin(HeadlessRenderPlugin)
            .add_plugin(DollPlugin);

        let doll_entity = app.world.spawn().insert(FirstName::default()).id();

        app.update();

        let named_entity = app.world.entity(doll_entity);
        assert!(named_entity.get::<Handle<Mesh>>().is_some());
        assert!(named_entity.get::<Handle<StandardMaterial>>().is_some());
    }
}
