use bevy::prelude::*;
use project_harmonia_base::game_world::{
    actor::SelectedActor,
    family::{FamilyMembers, FamilyMode},
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, ImageButtonBundle, Toggled},
    theme::Theme,
};

use crate::preview::Preview;

pub(super) struct MembersNodePlugin;

impl Plugin for MembersNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::select_actor.run_if(in_state(FamilyMode::Life)),
        );
    }
}

impl MembersNodePlugin {
    fn select_actor(
        mut commands: Commands,
        actor_buttons: Query<(Ref<Toggled>, &PlayActor), Changed<Toggled>>,
    ) {
        for (toggled, play_actor) in &actor_buttons {
            if toggled.0 && !toggled.is_added() {
                commands.entity(play_actor.0).insert(SelectedActor);
            }
        }
    }
}

pub(super) fn setup(
    parent: &mut ChildBuilder,
    theme: &Theme,
    members: &FamilyMembers,
    active_entity: Entity,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                align_self: AlignSelf::FlexEnd,
                column_gap: theme.gap.normal,
                padding: theme.padding.normal,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            for &entity in members.iter() {
                parent.spawn((
                    PlayActor(entity),
                    Preview::Actor(entity),
                    ExclusiveButton,
                    Toggled(entity == active_entity),
                    ImageButtonBundle::placeholder(theme),
                ));
            }
        });
}

#[derive(Component, Debug)]
struct PlayActor(Entity);
