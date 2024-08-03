pub(super) mod collection;
pub(super) mod material;
pub mod metadata;

use bevy::prelude::*;

use material::MaterialPlugin;
use metadata::MetadataPlugins;

pub(super) struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MaterialPlugin, MetadataPlugins));
    }
}
