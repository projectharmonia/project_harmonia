use bevy::prelude::*;

use crate::theme::Theme;

pub(super) struct LabelPlugin;

impl Plugin for LabelPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(theme);
    }
}

fn theme(
    trigger: Trigger<OnAdd, LabelKind>,
    theme: Res<Theme>,
    mut labels: Query<(&LabelKind, &mut TextFont, &mut TextColor)>,
) {
    let (label_kind, mut text_font, mut text_color) = labels
        .get_mut(trigger.entity())
        .expect("labels should be spawned with text or span");

    let label_theme = match label_kind {
        LabelKind::Small => &theme.label.small,
        LabelKind::Normal => &theme.label.normal,
        LabelKind::Large => &theme.label.large,
        LabelKind::Symbol => &theme.label.symbol,
    };

    text_font.font = label_theme.font.clone();
    text_font.font_size = label_theme.font_size;
    *text_color = label_theme.color;
}

#[derive(Component)]
#[require(Name(|| Name::new("Label")))]
pub enum LabelKind {
    Small,
    Normal,
    Large,
    Symbol,
}
