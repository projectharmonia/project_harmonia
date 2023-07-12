use bevy::prelude::*;

use crate::ui::theme::Theme;

pub(super) struct ProgressBarPlugin;

impl Plugin for ProgressBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems((Self::init_system, Self::progress_update_system));
    }
}

impl ProgressBarPlugin {
    fn init_system(
        mut commands: Commands,
        theme: Res<Theme>,
        progress_bars: Query<(Entity, &ProgressBar), Added<ProgressBar>>,
    ) {
        for (entity, progress_bar) in &progress_bars {
            commands.entity(entity).with_children(|parent| {
                parent.spawn(NodeBundle {
                    style: Style {
                        size: Size::width(Val::Percent(progress_bar.0)),
                        ..Default::default()
                    },
                    background_color: theme.progress_bar.fill_color.into(),
                    ..Default::default()
                });
            });
        }
    }

    /// Won't be triggered after spawning because button child will be spawned at the next frame.
    fn progress_update_system(
        progress_bars: Query<(&ProgressBar, &Children), Changed<ProgressBar>>,
        mut fill_nodes: Query<&mut Style>,
    ) {
        for (progress_bar, children) in &progress_bars {
            let mut iter = fill_nodes.iter_many_mut(children);
            let mut style = iter
                .fetch_next()
                .expect("progress bar should have child fill node");
            style.size.width = Val::Percent(progress_bar.0);
        }
    }
}

#[derive(Component)]
pub(crate) struct ProgressBar(pub(crate) f32);

#[derive(Bundle)]
pub(crate) struct ProgressBarBundle {
    progress_bar: ProgressBar,
    node_bundle: NodeBundle,
}

impl ProgressBarBundle {
    pub(crate) fn new(theme: &Theme, value: f32) -> Self {
        Self {
            progress_bar: ProgressBar(value),
            node_bundle: NodeBundle {
                style: theme.progress_bar.node.clone(),
                background_color: theme.progress_bar.background_color.into(),
                ..Default::default()
            },
        }
    }
}
