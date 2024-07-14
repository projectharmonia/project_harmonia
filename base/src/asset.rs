pub(super) mod collection;
pub mod metadata;

use bevy::prelude::*;

use self::metadata::MetadataPlugins;

pub(super) struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MetadataPlugins);
    }
}
