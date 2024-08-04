use bevy::prelude::*;
use project_harmonia_base::game_world::{
    family::{Budget, SelectedFamily},
    WorldState,
};
use project_harmonia_widgets::{label::LabelBundle, theme::Theme};

pub(super) struct PortraitNodePlugin;

impl Plugin for PortraitNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            Self::update_budget.run_if(in_state(WorldState::Family)),
        );
    }
}

impl PortraitNodePlugin {
    fn update_budget(
        families: Query<&Budget, (With<SelectedFamily>, Changed<Budget>)>,
        mut labels: Query<&mut Text, With<BudgetLabel>>,
    ) {
        if let Ok(budget) = families.get_single() {
            debug!("changing budget to `{budget:?}`");
            labels.single_mut().sections[0].value = budget.to_string();
        }
    }
}

pub(super) fn setup(parent: &mut ChildBuilder, theme: &Theme, budget: Budget) {
    parent
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(180.0),
                height: Val::Px(30.0),
                align_self: AlignSelf::FlexEnd,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            background_color: theme.panel_color.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn((BudgetLabel, LabelBundle::normal(theme, budget.to_string())));
        });
}

#[derive(Component)]
struct BudgetLabel;
