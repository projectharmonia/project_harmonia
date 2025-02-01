use bevy::prelude::*;
use project_harmonia_base::game_world::{
    family::{Budget, SelectedFamily},
    WorldState,
};
use project_harmonia_widgets::{label::LabelKind, theme::Theme};

pub(super) struct PortraitNodePlugin;

impl Plugin for PortraitNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_budget
                .never_param_warn()
                .run_if(in_state(WorldState::Family)),
        );
    }
}

fn update_budget(
    current_budget: Single<&Budget, (With<SelectedFamily>, Changed<Budget>)>,
    mut budget_label: Single<&mut Text, With<BudgetLabel>>,
) {
    debug!("changing budget to `{:?}`", **current_budget);
    ***budget_label = current_budget.to_string();
}

pub(super) fn setup(parent: &mut ChildBuilder, theme: &Theme, budget: Budget) {
    parent
        .spawn((
            Node {
                width: Val::Px(180.0),
                height: Val::Px(30.0),
                align_self: AlignSelf::FlexEnd,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            theme.panel_background,
        ))
        .with_children(|parent| {
            parent.spawn((BudgetLabel, Text::new(budget.to_string())));
        });
}

#[derive(Component)]
#[require(LabelKind(|| LabelKind::Normal))]
struct BudgetLabel;
