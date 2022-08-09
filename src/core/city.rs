use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{game_state::GameState, game_world::GameEntity};

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<City>()
            .add_system(Self::insert_spatial_system.run_in_state(GameState::InGame));
    }
}

impl CityPlugin {
    /// Insert [`SpatialBundle`] to make children visible.
    ///
    /// We delay the insertion to avoid serialization of components
    /// from [`SpatialBundle`] and spawn then on deserialization.
    fn insert_spatial_system(mut commands: Commands, added_cities: Query<Entity, Added<City>>) {
        for city in &added_cities {
            commands
                .entity(city)
                .insert_bundle(SpatialBundle::default());
        }
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    game_world: GameEntity,
}

impl CityBundle {
    #[cfg_attr(coverage, no_coverage)]
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            city: City,
            game_world: GameEntity,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct City;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_insertion() {
        let mut app = App::new();
        app.add_loopless_state(GameState::InGame)
            .add_plugin(CityPlugin);

        let city = app.world.spawn().insert_bundle(CityBundle::default()).id();

        app.update();

        assert!(
            app.world.entity(city).contains::<Transform>(),
            "Transform should be inserted into city on spawn"
        );
    }
}
