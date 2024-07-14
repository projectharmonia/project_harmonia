use bevy::prelude::*;

use crate::theme::Theme;

#[derive(Bundle)]
pub struct LabelBundle {
    label: Label,
    text_bundle: TextBundle,
}

impl LabelBundle {
    pub fn normal(theme: &Theme, text: impl Into<String>) -> Self {
        Self {
            label: Label,
            text_bundle: TextBundle::from_section(text, theme.label.normal.clone()),
        }
    }

    pub fn large(theme: &Theme, text: impl Into<String>) -> Self {
        Self {
            label: Label,
            text_bundle: TextBundle::from_section(text, theme.label.large.clone()),
        }
    }

    pub fn symbol(theme: &Theme, text: impl Into<String>) -> Self {
        Self {
            label: Label,
            text_bundle: TextBundle::from_section(text, theme.label.symbol.clone()),
        }
    }
}
