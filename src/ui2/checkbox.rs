use bevy::prelude::*;

use super::theme::Theme;

pub(super) struct CheckboxPlugin;

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
pub(super) struct Checkbox(pub(super) bool);

#[derive(Bundle)]
pub(super) struct CheckboxBundle {
    pub(super) checkbox: Checkbox,
    pub(super) button_bundle: ButtonBundle,
}
