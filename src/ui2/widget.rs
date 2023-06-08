pub(super) mod button;
pub(super) mod checkbox;
pub(super) mod ui_root;

use bevy::{prelude::*, ui::FocusPolicy};

use super::theme::Theme;
use button::ButtonPlugin;
use checkbox::CheckboxPlugin;
use ui_root::UiRootPlugin;

pub(super) struct WidgetPlugin;

impl Plugin for WidgetPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(ButtonPlugin)
            .add_plugin(CheckboxPlugin)
            .add_plugin(UiRootPlugin);
    }
}

#[derive(Bundle)]
pub(super) struct LabelBundle {
    label: Label,

    #[bundle]
    text_bundle: TextBundle,
}

impl LabelBundle {
    pub(super) fn new(theme: &Theme, text: impl Into<String>) -> Self {
        Self {
            label: Label,
            text_bundle: TextBundle::from_section(text, theme.text.normal.clone()),
        }
    }
}

#[derive(Bundle)]
pub(super) struct ModalBundle {
    modal: Modal,

    #[bundle]
    node_bundle: NodeBundle,
}

impl ModalBundle {
    pub(super) fn new(theme: &Theme) -> Self {
        Self {
            modal: Modal,
            node_bundle: NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    size: Size::all(Val::Percent(100.0)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                focus_policy: FocusPolicy::Block,
                background_color: theme.modal.background_color.into(),
                ..Default::default()
            },
        }
    }
}

#[derive(Component)]
pub(super) struct Modal;
