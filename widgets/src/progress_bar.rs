use bevy::prelude::*;

use super::theme::Theme;

pub(super) struct ProgressBarPlugin;

impl Plugin for ProgressBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(Self::init)
            .add_systems(PostUpdate, Self::update_progress);
    }
}

impl ProgressBarPlugin {
    fn init(
        trigger: Trigger<OnAdd, ProgressBar>,
        mut commands: Commands,
        theme: Res<Theme>,
        mut progress_bars: Query<&mut BackgroundColor>,
    ) {
        let mut background_color = progress_bars.get_mut(trigger.entity()).unwrap();
        *background_color = theme.progress_bar.background_color;

        commands
            .entity(trigger.entity())
            .with_child((Node::default(), theme.progress_bar.fill_color));
    }

    fn update_progress(
        progress_bars: Query<(&ProgressBar, &Children), Changed<ProgressBar>>,
        mut fill_nodes: Query<&mut Node>,
    ) {
        for (progress_bar, children) in &progress_bars {
            let mut iter = fill_nodes.iter_many_mut(children);
            let mut style = iter
                .fetch_next()
                .expect("progress bar should have child fill node");
            style.width = Val::Percent(progress_bar.0);
        }
    }
}

#[derive(Component)]
#[require(Node)]
pub struct ProgressBar(pub f32);
