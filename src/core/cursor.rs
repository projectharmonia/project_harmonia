pub(crate) mod cursor_object;

use bevy::prelude::*;
use iyes_loopless::prelude::*;

use cursor_object::CursorObjectPlugin;
use strum::{Display, EnumIter};

pub(super) struct CursorPlugins;

impl Plugin for CursorPlugins {
    fn build(&self, app: &mut App) {
        app.add_loopless_state(CursorMode::Objects)
            .add_plugin(CursorObjectPlugin);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Display, EnumIter)]
pub(crate) enum CursorMode {
    Objects,
    Lots,
}

impl CursorMode {
    pub(crate) fn glyph(self) -> &'static str {
        match self {
            Self::Objects => "🌳",
            Self::Lots => "🚧",
        }
    }
}
