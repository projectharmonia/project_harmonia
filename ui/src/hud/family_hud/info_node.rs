use bevy::prelude::*;
use project_harmonia_base::game_world::{
    actor::{
        needs::{Need, NeedGlyph},
        SelectedActor,
    },
    WorldState,
};
use project_harmonia_widgets::{
    button::{ButtonKind, TabContent, Toggled},
    label::LabelKind,
    progress_bar::ProgressBar,
    theme::Theme,
};
use strum::{EnumIter, IntoEnumIterator};

pub(super) struct InfoNodePlugin;

impl Plugin for InfoNodePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(cleanup_need_bars).add_systems(
            Update,
            update_need_bars.run_if(in_state(WorldState::Family)),
        );
    }
}

fn update_need_bars(
    mut commands: Commands,
    selected_actor: Single<(&Children, Ref<SelectedActor>)>,
    needs: Query<(Entity, &NeedGlyph, Ref<Need>)>,
    tabs: Query<(&TabContent, &InfoTab)>,
    mut progress_bars: Query<(&mut ProgressBar, &BarNeed)>,
) {
    let (children, selected_actor) = selected_actor.into_inner();
    let (tab_content, _) = tabs
        .iter()
        .find(|(_, &tab)| tab == InfoTab::Needs)
        .expect("tab with cities should be spawned on state enter");

    if selected_actor.is_added() {
        commands.entity(tab_content.0).despawn_descendants();
    }

    for (entity, glyph, need) in needs
        .iter_many(children)
        .filter(|(.., need)| need.is_changed() || selected_actor.is_added())
    {
        if let Some((mut progress_bar, _)) = progress_bars
            .iter_mut()
            .find(|(_, bar_need)| bar_need.0 == entity)
        {
            trace!("updating bar with `{need:?}` for `{entity}`");
            progress_bar.0 = need.0;
        } else {
            trace!("creating bar with `{need:?}` for `{entity}`");
            commands.entity(tab_content.0).with_children(|parent| {
                parent.spawn((LabelKind::Symbol, Text::new(glyph.0)));
                parent.spawn((BarNeed(entity), ProgressBar(need.0)));
            });
        }
    }
}

fn cleanup_need_bars(
    trigger: Trigger<OnRemove, Need>,
    mut commands: Commands,
    progress_bars: Query<(Entity, &BarNeed)>,
) {
    if let Some((entity, _)) = progress_bars
        .iter()
        .find(|(_, bar_need)| bar_need.0 == trigger.entity())
    {
        debug!("despawning bar `{entity}` for need `{}`", trigger.entity());
        commands.entity(entity).despawn_recursive();
    }
}

pub(super) fn setup(parent: &mut ChildBuilder, tab_commands: &mut Commands, theme: &Theme) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::ColumnReverse,
            position_type: PositionType::Absolute,
            align_self: AlignSelf::FlexEnd,
            right: Val::Px(0.0),
            ..Default::default()
        })
        .with_children(|parent| {
            let tabs_entity = parent
                .spawn((
                    Node {
                        padding: theme.padding.normal,
                        align_self: AlignSelf::FlexEnd,
                        ..Default::default()
                    },
                    theme.panel_background,
                ))
                .id();

            for (index, tab) in InfoTab::iter().enumerate() {
                let content_entity = match tab {
                    InfoTab::Skills => parent.spawn(Node::default()).id(),
                    InfoTab::Needs => parent
                        .spawn((
                            Node {
                                display: Display::Grid,
                                width: Val::Px(400.0),
                                column_gap: theme.gap.normal,
                                row_gap: theme.gap.normal,
                                padding: theme.padding.normal,
                                grid_template_columns: vec![
                                    GridTrack::auto(),
                                    GridTrack::flex(1.0),
                                    GridTrack::auto(),
                                    GridTrack::flex(1.0),
                                ],
                                ..Default::default()
                            },
                            theme.panel_background,
                        ))
                        .id(),
                };

                tab_commands
                    .spawn((
                        tab,
                        ButtonKind::Symbol,
                        TabContent(content_entity),
                        Toggled(index == 0),
                    ))
                    .with_child(Text::new(tab.glyph()))
                    .set_parent(tabs_entity);
            }
        });
}

#[derive(Component)]
struct BarNeed(Entity);

#[derive(Component, EnumIter, Clone, Copy, PartialEq)]
enum InfoTab {
    Skills,
    Needs,
}

impl InfoTab {
    fn glyph(self) -> &'static str {
        match self {
            InfoTab::Skills => "💡",
            InfoTab::Needs => "📈",
        }
    }
}
