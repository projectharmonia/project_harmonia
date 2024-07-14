pub mod button;
pub mod checkbox;
pub mod click;
pub mod progress_bar;
pub mod text_edit;
pub mod theme;

use bevy::{prelude::*, ui::FocusPolicy};

use button::ButtonPlugin;
use checkbox::CheckboxPlugin;
use click::ClickPlugin;
use progress_bar::ProgressBarPlugin;
use text_edit::TextEditPlugin;
use theme::{Theme, ThemePlugin};

pub struct WidgetsPlugin;

impl Plugin for WidgetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ButtonPlugin,
            CheckboxPlugin,
            ClickPlugin,
            ProgressBarPlugin,
            TextEditPlugin,
            ThemePlugin,
        ));
    }
}

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

#[derive(Bundle)]
pub struct DialogBundle {
    dialog: Dialog,
    interaction: Interaction,
    node_bundle: NodeBundle,
}

impl DialogBundle {
    pub fn new(theme: &Theme) -> Self {
        Self {
            dialog: Dialog,
            interaction: Default::default(),
            node_bundle: NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                focus_policy: FocusPolicy::Block,
                background_color: theme.modal_color.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Component)]
pub struct Dialog;
