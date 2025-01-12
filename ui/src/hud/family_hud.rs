mod building_hud;
mod info_node;
mod members_node;
mod portrait_node;
mod tasks_node;

use bevy::prelude::*;
use project_harmonia_base::{
    asset::manifest::object_manifest::ObjectManifest,
    game_world::{
        actor::{
            task::{ActiveTask, Task},
            SelectedActor,
        },
        family::{Budget, FamilyMembers, FamilyMode, FamilyPlugin, SelectedFamily},
        WorldState,
    },
};
use project_harmonia_widgets::{
    button::{ButtonKind, TabContent, Toggled},
    theme::Theme,
};
use strum::IntoEnumIterator;

use building_hud::BuildingHudPlugin;
use info_node::InfoNodePlugin;
use portrait_node::PortraitNodePlugin;
use tasks_node::TasksNodePlugin;

pub(super) struct FamilyHudPlugin;

impl Plugin for FamilyHudPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            TasksNodePlugin,
            InfoNodePlugin,
            PortraitNodePlugin,
            BuildingHudPlugin,
        ))
        .add_systems(
            OnEnter(WorldState::Family),
            Self::setup.after(FamilyPlugin::select),
        );
    }
}

impl FamilyHudPlugin {
    fn setup(
        mut commands: Commands,
        mut tab_commands: Commands,
        theme: Res<Theme>,
        object_manifests: Res<Assets<ObjectManifest>>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
        actor_children: Single<&Children, With<SelectedActor>>,
        selected_family: Single<(&Budget, &FamilyMembers), With<SelectedFamily>>,
        selected_entity: Single<Entity, With<SelectedActor>>,
        tasks: Query<(Entity, Has<ActiveTask>), With<Task>>,
    ) {
        debug!("showing family hud");
        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn((
                    PickingBehavior::IGNORE,
                    StateScoped(WorldState::Family),
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    let tabs_entity = parent
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                right: Val::Px(0.0),
                                padding: theme.padding.normal,
                                ..Default::default()
                            },
                            theme.panel_background,
                        ))
                        .id();

                    for mode in FamilyMode::iter() {
                        let content_entity = parent
                            .spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    ..Default::default()
                                },
                                PickingBehavior::IGNORE,
                            ))
                            .with_children(|parent| match mode {
                                FamilyMode::Life => {
                                    tasks_node::setup(parent, &theme, *actor_children, &tasks);

                                    let (&budget, members) = *selected_family;
                                    portrait_node::setup(parent, &theme, budget);
                                    members_node::setup(parent, &theme, members, *selected_entity);
                                    info_node::setup(parent, &mut tab_commands, &theme);
                                }
                                FamilyMode::Building => building_hud::setup(
                                    parent,
                                    &mut tab_commands,
                                    &theme,
                                    &object_manifests,
                                ),
                            })
                            .id();

                        tab_commands
                            .spawn((
                                mode,
                                ButtonKind::Symbol,
                                TabContent(content_entity),
                                Toggled(mode == Default::default()),
                            ))
                            .with_child(Text::new(mode.glyph()))
                            .set_parent(tabs_entity)
                            .observe(Self::set_family_mode);
                    }
                });
        });
    }

    fn set_family_mode(
        trigger: Trigger<Pointer<Click>>,
        mut commands: Commands,
        buttons: Query<&FamilyMode>,
    ) {
        let mode = *buttons.get(trigger.entity()).unwrap();
        info!("changing family mode to `{mode:?}`");
        commands.set_state(mode);
    }
}
