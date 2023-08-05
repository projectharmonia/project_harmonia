mod tell_secret;

use bevy::{app::PluginGroupBuilder, prelude::*};

use tell_secret::TellSecretPlugin;

pub(super) struct FriendlyPlugins;

impl PluginGroup for FriendlyPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(TellSecretPlugin)
    }
}
