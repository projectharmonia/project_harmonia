use bevy::prelude::*;
use iyes_loopless::prelude::*;

pub(super) struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(GameState::Menu)
            .add_exit_system(GameState::InGame, Self::cleanup_ingame_entities_system);
    }
}

impl GameStatePlugin {
    fn cleanup_ingame_entities_system(
        mut commands: Commands,
        ingame_entities: Query<Entity, With<InGameOnly>>,
    ) {
        for entity in ingame_entities.iter() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

/// All entities with this component will be removed after leaving [`InGame`] state
#[derive(Component, Default)]
pub(super) struct InGameOnly;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) enum GameState {
    Menu,
    InGame,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingame_entities_cleanup() {
        let mut app = App::new();
        app.add_plugin(GameStatePlugin);

        app.world.insert_resource(NextState(GameState::InGame));
        let child_entity = app.world.spawn().id();
        let ingame_entity = app
            .world
            .spawn()
            .insert(InGameOnly)
            .push_children(&[child_entity])
            .id();

        app.update();

        app.world.insert_resource(NextState(GameState::Menu));

        app.update();

        assert!(
            app.world.get_entity(ingame_entity).is_none(),
            "Ingame entity should be despawned after leaving ingame state"
        );
        assert!(
            app.world.get_entity(child_entity).is_none(),
            "Children of ingame entity should be despawned with its parent"
        );
    }
}
