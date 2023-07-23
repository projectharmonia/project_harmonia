use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_replicon::prelude::*;

use crate::core::game_world::WorldName;

pub(super) struct NeedsPlugin;

impl Plugin for NeedsPlugin {
    fn build(&self, app: &mut App) {
        app.replicate::<Hunger>()
            .replicate::<Social>()
            .replicate::<Hygiene>()
            .replicate::<Fun>()
            .replicate::<Energy>()
            .replicate::<Bladder>()
            .replicate::<Need>()
            .not_replicate_if_present::<Name, Need>()
            .add_systems(
                PreUpdate,
                Self::tick_system
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(has_authority()),
            )
            .add_systems(
                PostUpdate, // To initialize after actor spawn.
                (
                    Self::hunger_init_system,
                    Self::social_init_system,
                    Self::hygiene_init_system,
                    Self::fun_init_system,
                    Self::energy_init_system,
                    Self::bladder_init_system,
                )
                    .run_if(resource_exists::<WorldName>()),
            );
    }
}

impl NeedsPlugin {
    fn hunger_init_system(mut commands: Commands, needs: Query<Entity, Added<Hunger>>) {
        for entity in &needs {
            commands
                .entity(entity)
                .insert((Name::new("Hunger"), NeedGlyph("🍴"), NeedRate(-0.4)));
        }
    }

    fn social_init_system(mut commands: Commands, needs: Query<Entity, Added<Social>>) {
        for entity in &needs {
            commands
                .entity(entity)
                .insert((Name::new("Social"), NeedGlyph("💬"), NeedRate(-0.1)));
        }
    }

    fn hygiene_init_system(mut commands: Commands, needs: Query<Entity, Added<Hygiene>>) {
        for entity in &needs {
            commands
                .entity(entity)
                .insert((Name::new("Hygiene"), NeedGlyph("🚿"), NeedRate(-0.3)));
        }
    }

    fn fun_init_system(mut commands: Commands, needs: Query<Entity, Added<Fun>>) {
        for entity in &needs {
            commands
                .entity(entity)
                .insert((Name::new("Fun"), NeedGlyph("🎉"), NeedRate(-0.1)));
        }
    }

    fn energy_init_system(mut commands: Commands, needs: Query<Entity, Added<Energy>>) {
        for entity in &needs {
            commands
                .entity(entity)
                .insert((Name::new("Energy"), NeedGlyph("🔋"), NeedRate(-0.2)));
        }
    }

    fn bladder_init_system(mut commands: Commands, needs: Query<Entity, Added<Bladder>>) {
        for entity in &needs {
            commands
                .entity(entity)
                .insert((Name::new("Bladder"), NeedGlyph("🚽"), NeedRate(-0.5)));
        }
    }

    fn tick_system(mut needs: Query<(&mut Need, &NeedRate)>) {
        for (mut need, rate) in &mut needs {
            if need.0 > rate.0 {
                need.0 += rate.0;
            } else {
                need.0 = 0.0;
            }
        }
    }
}

#[derive(Bundle)]
pub(crate) struct NeedBundle<T: Component> {
    need: Need,
    marker: T,
    parent_sync: ParentSync,
    replication: Replication,
}

impl<T: Component + Default> Default for NeedBundle<T> {
    fn default() -> Self {
        Self {
            need: Default::default(),
            marker: T::default(),
            parent_sync: Default::default(),
            replication: Replication,
        }
    }
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Hunger;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Social;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Hygiene;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Fun;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Energy;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub(crate) struct Bladder;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct Need(pub(crate) f32);

impl Default for Need {
    fn default() -> Self {
        Self(100.0)
    }
}

#[derive(Component)]
struct NeedRate(f32);

#[derive(Component)]
pub(crate) struct NeedGlyph(pub(crate) &'static str);
