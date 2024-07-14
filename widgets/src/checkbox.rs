use bevy::prelude::*;

use super::{
    click::{Click, LastInteraction},
    theme::Theme,
};

pub(crate) struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (Self::init, Self::toggle, Self::update_tick));
    }
}

impl CheckboxPlugin {
    fn init(
        mut commmands: Commands,
        theme: Res<Theme>,
        checkboxes: Query<(Entity, &Checkbox, &CheckboxText), Added<CheckboxText>>,
    ) {
        for (entity, checkbox, text) in &checkboxes {
            commmands.entity(entity).with_children(|parent| {
                parent
                    .spawn((
                        LastInteraction::default(),
                        ButtonBundle {
                            style: theme.checkbox.button.clone(),
                            ..Default::default()
                        },
                    ))
                    .with_children(|parent| {
                        if checkbox.0 {
                            parent.spawn(NodeBundle {
                                style: theme.checkbox.tick.clone(),
                                background_color: theme.checkbox.tick_color.into(),
                                ..Default::default()
                            });
                        }
                    });
                parent.spawn(TextBundle::from_section(
                    text.0.clone(),
                    theme.label.normal.clone(),
                ));
            });
        }
    }

    fn toggle(
        mut click_events: EventReader<Click>,
        mut checkboxes: Query<&mut Checkbox>,
        parents: Query<&Parent>,
    ) {
        for parent in parents.iter_many(click_events.read().map(|event| event.0)) {
            if let Ok(mut checkbox) = checkboxes.get_mut(**parent) {
                checkbox.0 = !checkbox.0;
            }
        }
    }

    /// Won't be triggered after spawning because button child will be spawned at the next frame.
    fn update_tick(
        mut commmands: Commands,
        theme: Res<Theme>,
        checkboxes: Query<(&Children, &Checkbox), Changed<Checkbox>>,
        buttons: Query<Entity, With<Button>>,
    ) {
        for (chidlren, checkbox) in &checkboxes {
            let entity = buttons
                .iter_many(chidlren)
                .next()
                .expect("checkbox should have child button");
            if checkbox.0 {
                commmands.entity(entity).with_children(|parent| {
                    parent.spawn(NodeBundle {
                        style: theme.checkbox.tick.clone(),
                        background_color: theme.checkbox.tick_color.into(),
                        ..Default::default()
                    });
                });
            } else {
                commmands.entity(entity).despawn_descendants();
            }
        }
    }
}

#[derive(Component)]
pub struct Checkbox(pub bool);

#[derive(Component)]
pub struct CheckboxText(pub String);

#[derive(Bundle)]
pub struct CheckboxBundle {
    checkbox: Checkbox,
    checkbox_text: CheckboxText,
    last_interaction: LastInteraction,
    node_bundle: NodeBundle,
}

impl CheckboxBundle {
    pub fn new(theme: &Theme, checked: bool, text: impl Into<String>) -> Self {
        Self {
            checkbox: Checkbox(checked),
            checkbox_text: CheckboxText(text.into()),
            last_interaction: Default::default(),
            node_bundle: NodeBundle {
                style: theme.checkbox.node.clone(),
                ..Default::default()
            },
        }
    }
}
