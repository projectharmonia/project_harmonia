use bevy::prelude::*;
use project_harmonia_widgets::{
    button::{ExclusiveButton, TextButtonBundle, Toggled},
    theme::Theme,
};

pub(super) fn setup(parent: &mut ChildBuilder, theme: &Theme) {
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|parent| {
            // Just a stab for instruments.
            parent.spawn((
                ExclusiveButton,
                Toggled(true),
                TextButtonBundle::symbol(theme, "➕"),
            ));
        });
}
