mod building_hud;
mod info_node;
mod members_node;
mod portrait_node;
mod tasks_node;

use bevy::prelude::*;
use project_harmonia_base::{
    asset::info::object_info::ObjectInfo,
    game_world::{
        actor::SelectedActor,
        family::{Budget, FamilyMembers, FamilyMode, FamilyPlugin, SelectedFamily},
        WorldState,
    },
};
use project_harmonia_widgets::{
    button::{ExclusiveButton, TabContent, TextButtonBundle, Toggled},
    theme::Theme,
};
use strum::IntoEnumIterator;

use building_hud::BuildingHudPlugin;
use info_node::InfoNodePlugin;
use members_node::MembersNodePlugin;
use portrait_node::PortraitNodePlugin;
use tasks_node::TasksNodePlugin;

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TasksNodePlugin,
            InfoNodePlugin,
            PortraitNodePlugin,
            MembersNodePlugin,
            BuildingHudPlugin,
        ))
        .add_systems(
            OnEnter(WorldState::Family),
            Self::setup.after(FamilyPlugin::select),
        )
        .add_systems(
            Update,
            Self::set_family_mode.run_if(in_state(WorldState::Family)),
        );
    }
}

impl FamilyHudPlugin {
    fn setup(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        objects_info: Res<Assets<ObjectInfo>>,
        families: Query<(&Budget, &FamilyMembers), With<SelectedFamily>>,
        actors: Query<Entity, With<SelectedActor>>,
        roots: Query<Entity, (With<Node>, Without<Parent>)>,
    ) {
        debug!("showing family hud");
        commands.entity(roots.single()).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(WorldState::Family),
                    NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    let tabs_entity = parent
                        .spawn(NodeBundle {
                            style: Style {
                                position_type: PositionType::Absolute,
                                right: Val::Px(0.0),
                                padding: theme.padding.normal,
                                ..Default::default()
                            },
                            background_color: theme.panel_color.into(),
                            ..Default::default()
                        })
                        .id();

                    for mode in FamilyMode::iter() {
                        let content_entity = parent
                            .spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    ..Default::default()
                                },
                                ..Default::default()
                            })
                            .with_children(|parent| match mode {
                                FamilyMode::Life => {
                                    tasks_node::setup(parent, &theme);

                                    let (&budget, members) = families.single();
                                    portrait_node::setup(parent, &theme, budget);
                                    members_node::setup(parent, &theme, members, actors.single());
                                    info_node::setup(parent, &mut tab_commands, &theme);
                                }
                                FamilyMode::Building => building_hud::setup(
                                    parent,
                                    &mut tab_commands,
                                    &theme,
                                    &objects_info,
                                ),
                            })
                            .id();

                        tab_commands
                            .spawn((
                                mode,
                                TabContent(content_entity),
                                ExclusiveButton,
                                Toggled(mode == Default::default()),
                                TextButtonBundle::symbol(&theme, mode.glyph()),
                            ))
                            .set_parent(tabs_entity);
                    }
                });
        });
    }

    fn set_family_mode(
        mut family_mode: ResMut<NextState<FamilyMode>>,
        buttons: Query<(Ref<Toggled>, &FamilyMode), Changed<Toggled>>,
    ) {
        for (toggled, &mode) in &buttons {
            if toggled.0 && !toggled.is_added() {
                info!("changing family mode to `{mode:?}`");
                family_mode.set(mode);
            }
        }
    }
}
