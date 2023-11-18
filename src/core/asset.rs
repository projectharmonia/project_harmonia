pub(super) mod collection;
pub(crate) mod metadata;

use bevy::prelude::*;

use self::metadata::MetadataPlugin;

pub(super) struct AssetPlugin;

impl Plugin for AssetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MetadataPlugin);
    }
}
