use bevy::{prelude::*, reflect::GetTypeRegistration};
use bevy_replicon::prelude::*;
use bevy_trait_query::{queryable, RegisterExt};

pub(super) struct NeedsPlugin;

impl Plugin for NeedsPlugin {
    fn build(&self, app: &mut App) {
        app.register_need::<Hunger>()
            .register_need::<Social>()
            .register_need::<Hygiene>()
            .register_need::<Fun>()
            .register_need::<Energy>()
            .register_need::<Bladder>()
            .add_system(Self::tick_system.in_set(ServerSet::Authority));
    }
}

impl NeedsPlugin {
    fn tick_system(time: Res<Time>, mut actors: Query<&mut dyn Need>) {
        for needs in &mut actors {
            for mut need in needs {
                need.update(time.delta_seconds());
            }
        }
    }
}

trait NeedExt {
    fn register_need<T: Need + GetTypeRegistration + Component>(&mut self) -> &mut Self;
}

impl NeedExt for App {
    fn register_need<T: Need + GetTypeRegistration + Component>(&mut self) -> &mut Self {
        self.replicate::<T>().register_component_as::<dyn Need, T>()
    }
}

#[queryable]
pub(crate) trait Need {
    fn glyph(&self) -> &'static str;
    fn value(&self) -> f32;
    fn update(&mut self, delta: f32);
}

#[macro_export]
macro_rules! define_need {
    ($name: ident, $rate: literal, $glyph: literal) => {
        #[derive(Component, Reflect)]
        #[reflect(Component)]
        pub(crate) struct $name {
            value: f32,
            rate: f32,
        }

        impl Default for $name {
            fn default() -> Self {
                $name {
                    value: 100.0,
                    rate: $rate,
                }
            }
        }

        impl Need for $name {
            fn glyph(&self) -> &'static str {
                $glyph
            }

            fn value(&self) -> f32 {
                self.value
            }

            fn update(&mut self, delta_secs: f32) {
                self.value += self.rate * delta_secs;
            }
        }
    };
}

define_need!(Hunger, -1.0, "🍽");
define_need!(Social, -1.0, "💬");
define_need!(Hygiene, -1.0, "🚿");
define_need!(Fun, -1.0, "🎉");
define_need!(Energy, -1.0, "🔋");
define_need!(Bladder, -1.0, "🚽");
