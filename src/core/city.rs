use bevy::prelude::*;
use bevy_atmosphere::prelude::*;
use derive_more::Display;
use iyes_loopless::prelude::*;
use strum::EnumIter;

use super::{
    doll::ActiveDoll,
    game_state::GameState,
    game_world::GameWorld,
    network::replication::replication_rules::{AppReplicationExt, Replication},
    orbit_camera::{OrbitCameraBundle, OrbitOrigin},
};

/// To flush activation / deactivation commands after [`CoreStage::PostUpdate`].
#[derive(StageLabel)]
struct CityVisiblilityStage;

pub(super) struct CityPlugin;

impl Plugin for CityPlugin {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(CityMode::Objects)
            .register_and_replicate::<City>()
            .not_replicate_if_present::<Transform, City>()
            .init_resource::<PlacedCities>()
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
    /// City square side size.
    pub(super) const CITY_SIZE: f32 = 100.0;

    /// Inserts [`TransformBundle`] and places cities next to each other.
    fn init_system(
        mut commands: Commands,
        mut placed_citites: ResMut<PlacedCities>,
        added_cities: Query<Entity, Added<City>>,
    ) {
        for entity in &added_cities {
            let transform =
                Transform::from_translation(Vec3::X * Self::CITY_SIZE * placed_citites.0 as f32);
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
        if let Some(entity) = deactivated_dolls.iter().next() {
            let parent = parents
                .get(entity)
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
    ) {
        if let Ok((entity, mut visibility)) = active_cities.get_single_mut() {
            visibility.is_visible = true;
            commands.entity(entity).with_children(|parent| {
                parent.spawn((OrbitCameraBundle::default(), AtmosphereCamera::default()));
            });
        }
    }

    fn visibility_disable_system(
        mut commands: Commands,
        deactivated_cities: RemovedComponents<ActiveCity>,
        mut visibility: Query<&mut Visibility>,
        cameras: Query<Entity, With<OrbitOrigin>>,
    ) {
        if let Some(entity) = deactivated_cities.iter().next() {
            let mut visibility = visibility
                .get_mut(entity)
                .expect("city should always have a visibility component");
            visibility.is_visible = false;
            commands.entity(entity).remove::<ActiveCity>();
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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter)]
pub(crate) enum CityMode {
    Objects,
    Lots,
}

impl CityMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "🌳",
            Self::Lots => "🚧",
        }
    }
}

#[derive(Bundle, Default)]
pub(crate) struct CityBundle {
    name: Name,
    city: City,
    replication: Replication,
}

impl CityBundle {
    pub(crate) fn new(name: Name) -> Self {
        Self {
            name,
            city: City,
            replication: Replication,
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
