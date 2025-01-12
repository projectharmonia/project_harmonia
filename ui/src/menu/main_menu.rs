use bevy::{app::AppExit, prelude::*};

use super::{settings_menu::SettingsMenuOpen, MenuState};
use project_harmonia_widgets::{button::ButtonKind, theme::Theme};

pub(super) struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(MenuState::MainMenu), Self::setup);
    }
}

impl MainMenuPlugin {
    fn setup(
        mut commands: Commands,
        theme: Res<Theme>,
        root_entity: Single<Entity, (With<Node>, Without<Parent>)>,
    ) {
        info!("entering main menu");
        commands.entity(*root_entity).with_children(|parent| {
            parent
                .spawn((
                    StateScoped(MenuState::MainMenu),
                    Node {
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        align_items: AlignItems::FlexStart,
                        justify_content: JustifyContent::Center,
                        padding: theme.padding.global,
                        row_gap: theme.gap.large,
                        ..Default::default()
                    },
                ))
                .with_children(|parent| {
                    parent
                        .spawn(ButtonKind::Large)
                        .with_child(Text::new("Play"))
                        .observe(Self::play);
                    parent
                        .spawn(ButtonKind::Large)
                        .with_child(Text::new("Settings"))
                        .observe(Self::open_settings);

                    parent
                        .spawn(ButtonKind::Large)
                        .with_child(Text::new("Exit"))
                        .observe(Self::exit);
                });
        });
    }

    fn play(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
        commands.set_state(MenuState::WorldBrowser);
    }

    fn open_settings(_trigger: Trigger<Pointer<Click>>, mut commands: Commands) {
        commands.trigger(SettingsMenuOpen);
    }

    fn exit(_trigger: Trigger<Pointer<Click>>, mut exit_events: EventWriter<AppExit>) {
        info!("exiting game");
        exit_events.send_default();
    }
}
