use bevy::{ecs::system::EntityCommands, prelude::*};

use crate::ui2::theme::Theme;

pub(crate) struct CheckboxPlugin;

impl Plugin for CheckboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((Self::interaction_system, Self::tick_system));
    }
}

impl CheckboxPlugin {
    fn interaction_system(
        mut checkboxes: Query<(&Interaction, &mut Checkbox), Changed<Interaction>>,
    ) {
        for (interaction, mut checkbox) in &mut checkboxes {
            if *interaction == Interaction::Clicked {
                checkbox.0 = !checkbox.0;
            }
        }
    }

    fn tick_system(
        mut commmands: Commands,
        theme: Res<Theme>,
        checkboxes: Query<(Entity, &Checkbox), Changed<Checkbox>>,
    ) {
        for (entity, checkbox) in &checkboxes {
            if checkbox.0 {
                commmands.entity(entity).despawn_descendants();
            } else {
                commmands.entity(entity).with_children(|parent| {
                    parent.spawn(NodeBundle {
                        style: theme.checkbox.tick.clone(),
                        background_color: theme.checkbox.tick_color.into(),
                        ..Default::default()
                    });
                });
            }
        }
    }
}

#[derive(Component)]
pub(crate) struct Checkbox(pub(crate) bool);

pub(crate) trait CheckboxCommandsExt<'w, 's> {
    fn spawn_checkbox(
        &mut self,
        theme: &Theme,
        checked: bool,
        text: impl Into<String>,
        bundle: impl Bundle,
    ) -> EntityCommands<'w, 's, '_>;
}

impl<'w, 's> CheckboxCommandsExt<'w, 's> for ChildBuilder<'w, 's, '_> {
    fn spawn_checkbox(
        &mut self,
        theme: &Theme,
        checked: bool,
        text: impl Into<String>,
        bundle: impl Bundle,
    ) -> EntityCommands<'w, 's, '_> {
        let mut entity = self.spawn((
            bundle,
            NodeBundle {
                style: theme.checkbox.node.clone(),
                ..Default::default()
            },
        ));
        entity.with_children(|parent| {
            parent.spawn((
                Checkbox(checked),
                ButtonBundle {
                    style: theme.checkbox.button.clone(),
                    ..Default::default()
                },
            ));
            parent.spawn(TextBundle::from_section(text, theme.text.normal.clone()));
        });
        entity
    }
}
