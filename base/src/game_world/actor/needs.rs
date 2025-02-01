use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};

pub(super) struct NeedsPlugin;

impl Plugin for NeedsPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Hunger>()
            .register_type::<Social>()
            .register_type::<Hygiene>()
            .register_type::<Fun>()
            .register_type::<Energy>()
            .register_type::<Bladder>()
            .register_type::<Need>()
            .replicate::<Hunger>()
            .replicate::<Social>()
            .replicate::<Hygiene>()
            .replicate::<Fun>()
            .replicate::<Energy>()
            .replicate::<Bladder>()
            .replicate::<Need>()
            .add_systems(
                Update,
                update_values
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(server_or_singleplayer),
            );
    }
}

fn update_values(mut needs: Query<(&mut Need, &NeedRate)>) {
    for (mut need, rate) in &mut needs {
        if need.0 > rate.0 {
            need.0 += rate.0;
        } else {
            need.0 = 0.0;
        }
    }
}

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(
    Need,
    NeedGlyph(|| NeedGlyph("🍴")),
    NeedRate(|| NeedRate(-0.4)),
)]
pub(crate) struct Hunger;

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(
    Need,
    NeedGlyph(|| NeedGlyph("💬")),
    NeedRate(|| NeedRate(-0.1)),
)]
pub(crate) struct Social;

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(
    Need,
    NeedGlyph(|| NeedGlyph("🚿")),
    NeedRate(|| NeedRate(-0.3)),
)]
pub(crate) struct Hygiene;

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(
    Need,
    NeedGlyph(|| NeedGlyph("🎉")),
    NeedRate(|| NeedRate(-0.1)),
)]
pub(crate) struct Fun;

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(
    Need,
    NeedGlyph(|| NeedGlyph("🔋")),
    NeedRate(|| NeedRate(-0.2)),
)]
pub(crate) struct Energy;

#[derive(Component, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(
    Need,
    NeedGlyph(|| NeedGlyph("🚽")),
    NeedRate(|| NeedRate(-0.5)),
)]
pub(crate) struct Bladder;

#[derive(Component, Debug, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
#[require(ParentSync, Replicated)]
pub struct Need(pub f32);

impl Default for Need {
    fn default() -> Self {
        Self(100.0)
    }
}

#[derive(Component)]
struct NeedRate(f32);

#[derive(Component)]
pub struct NeedGlyph(pub &'static str);
