pub(super) mod collection;
pub mod info;
pub(super) mod material;

use bevy::prelude::*;

use info::InfoPlugins;
use material::MaterialPlugin;

pub(super) struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin, InfoPlugins));
    }
}
