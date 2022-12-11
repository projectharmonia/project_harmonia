use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{
    doll::ActiveDoll,
    game_state::GameState,
    game_world::{GameEntity, GameWorld},
    orbit_camera::{OrbitCameraBundle, OrbitOrigin},
    settings::Settings,
};

/// To flush activation / deactivation commands after [`CoreStage::PostUpdate`].
#[derive(StageLabel)]
struct CityVisiblilityStage;

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlacedCities>()
            .register_type::<City>()
            .add_stage_after(
                CoreStage::PostUpdate,
                CityVisiblilityStage,
                SystemStage::parallel(),
            )
            .add_system(Self::init_system.run_if_resource_exists::<GameWorld>())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::doll_activation_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::doll_deactivation_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_exit_system(GameState::City, Self::deactivation_system)
            .add_system_to_stage(
                CityVisiblilityStage,
                Self::visibility_enable_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_system_to_stage(
                CityVisiblilityStage,
                Self::visibility_disable_system.run_if_resource_exists::<GameWorld>(),
            )
            .add_system(Self::cleanup_system.run_if_resource_removed::<GameWorld>())
            .add_system(Self::placed_cities_reset_system.run_if_resource_removed::<GameWorld>());
    }
}

impl CityPlugin {
    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn init_system(
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, Added<City>>,
    ) {
        const CITY_SIZE: f32 = 100.0;
        for entity in &added_cities {
            let transform =
                Transform::from_translation(Vec3::X * CITY_SIZE * placed_citites.0 as f32);
            commands.entity(entity).insert((
                TransformBundle::from_transform(transform),
                VisibilityBundle {
                    visibility: Visibility { is_visible: false },
                    ..Default::default()
                },
            ));
            placed_citites.0 += 1;
        }
    }

    fn doll_activation_system(
        mut commands: Commands,
        new_active_dolls: Query<&Parent, Added<ActiveDoll>>,
    ) {
        if let Ok(parent) = new_active_dolls.get_single() {
            commands.entity(parent.get()).insert(ActiveCity);
        }
    }

    fn doll_deactivation_system(
        mut commands: Commands,
        deactivated_dolls: RemovedComponents<ActiveDoll>,
        parents: Query<&Parent>,
    ) {
        if let Some(doll_entity) = deactivated_dolls.iter().next() {
            let parent = parents
                .get(doll_entity)
                .expect("deactivated doll should have a family");
            commands.entity(parent.get()).remove::<ActiveCity>();
        }
    }

    fn deactivation_system(mut commands: Commands, active_cities: Query<Entity, With<ActiveCity>>) {
        commands
            .entity(active_cities.single())
            .remove::<ActiveCity>();
    }

    fn visibility_enable_system(
        mut commands: Commands,
        mut active_cities: Query<(Entity, &mut Visibility), Added<ActiveCity>>,
        settings: Res<Settings>,
    ) {
        if let Ok((city_entity, mut visibility)) = active_cities.get_single_mut() {
            visibility.is_visible = true;
            commands.entity(city_entity).with_children(|parent| {
                parent.spawn(OrbitCameraBundle::new(settings.video.render_graph_name()));
            });
        }
    }

    fn visibility_disable_system(
        mut commands: Commands,
        deactivated_cities: RemovedComponents<ActiveCity>,
        mut visibility: Query<&mut Visibility>,
        cameras: Query<Entity, With<OrbitOrigin>>,
    ) {
        if let Some(city_entity) = deactivated_cities.iter().next() {
            let mut visibility = visibility
                .get_mut(city_entity)
                .expect("city should always have a visibility component");
            visibility.is_visible = false;
            commands.entity(city_entity).remove::<ActiveCity>();
            commands.entity(cameras.single()).despawn();
        }
    }

    /// Removes all cities and their children.
    fn cleanup_system(mut commands: Commands, cities: Query<Entity, With<City>>) {
        for entity in &cities {
            commands.entity(entity).despawn_recursive();
        }
    }

    /// Resets [`PlacedCities`] counter to 0.
    fn placed_cities_reset_system(mut placed_citites: ResMut<PlacedCities>) {
        placed_citites.0 = 0;
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    game_world: GameEntity,
}

impl CityBundle {
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

#[derive(Component)]
pub(crate) struct ActiveCity;

/// Number of placed cities.
///
/// The number increases when a city is placed, but does not decrease
/// when it is removed to assign a unique position to new each city.
#[derive(Default, Resource)]
struct PlacedCities(usize);
