use bevy::prelude::*;
use project_harmonia_base::game_world::{actor::SelectedActor, family::FamilyMembers};
use project_harmonia_widgets::{
    button::{ButtonKind, ExclusiveButton, Toggled},
    theme::Theme,
};

use crate::preview::Preview;

pub(super) fn setup(
    parent: &mut ChildBuilder,
    theme: &Theme,
    members: &FamilyMembers,
    active_entity: Entity,
) {
    parent
        .spawn((
            Node {
                align_self: AlignSelf::FlexEnd,
                column_gap: theme.gap.normal,
                padding: theme.padding.normal,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            for &entity in members.iter() {
                parent
                    .spawn((
                        ButtonKind::Image,
                        ExclusiveButton,
                        Toggled(entity == active_entity),
                    ))
                    .with_child(Preview::Actor(entity))
                    .observe(
                        move |_trigger: Trigger<Pointer<Click>>, mut commands: Commands, selected_entity: Single<Entity, With<SelectedActor>>| {
                            commands.entity(*selected_entity).remove::<SelectedActor>();
                            commands.entity(entity).insert(SelectedActor);
                        },
                    );
            }
        });
}
