use bevy::prelude::*;

use crate::ui2::theme::Theme;

pub(crate) struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((
            Self::init_system,
            Self::interaction_system,
            Self::tick_system,
        ));
    }
}

impl CheckboxPlugin {
    fn init_system(
        mut commmands: Commands,
        theme: Res<Theme>,
        checkboxes: Query<(Entity, &Checkbox, &CheckboxText), Added<CheckboxText>>,
    ) {
        for (entity, checkbox, text) in &checkboxes {
            commmands.entity(entity).with_children(|parent| {
                parent
                    .spawn(ButtonBundle {
                        style: theme.checkbox.button.clone(),
                        ..Default::default()
                    })
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
                    theme.text.normal.clone(),
                ));
            });
        }
    }

    fn interaction_system(
        mut checkboxes: Query<&mut Checkbox>,
        interactions: Query<(&Parent, &Interaction), Changed<Interaction>>,
    ) {
        for (parent, interaction) in &interactions {
            if *interaction == Interaction::Clicked {
                if let Ok(mut checkbox) = checkboxes.get_mut(**parent) {
                    checkbox.0 = !checkbox.0;
                }
            }
        }
    }

    /// Won't be triggered after spawning because button child will be spawned at the next frame.
    fn tick_system(
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
pub(crate) struct Checkbox(pub(crate) bool);

#[derive(Component)]
pub(crate) struct CheckboxText(pub(crate) String);

#[derive(Bundle)]
pub(crate) struct CheckboxBundle {
    checkbox: Checkbox,
    checkbox_text: CheckboxText,

    #[bundle]
    node_bundle: NodeBundle,
}

impl CheckboxBundle {
    pub(crate) fn new(theme: &Theme, checked: bool, text: impl Into<String>) -> Self {
        Self {
            checkbox: Checkbox(checked),
            checkbox_text: CheckboxText(text.into()),
            node_bundle: NodeBundle {
                style: theme.checkbox.node.clone(),
                ..Default::default()
            },
        }
    }
}
