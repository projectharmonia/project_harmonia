use bevy::prelude::*;
use iyes_loopless::prelude::*;

use super::{
    doll::{ActiveDoll, FirstName, LastName},
    game_state::GameState,
    orbit_camera::OrbitCameraBundle,
    settings::Settings,
};

pub(super) struct FamilyEditorPlugin;

impl Plugin for FamilyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FamilyReset>()
            .add_enter_system(GameState::FamilyEditor, Self::spawn_system)
            .add_exit_system(GameState::FamilyEditor, Self::cleanup_system)
            .add_system(Self::reset_family_system.run_on_event::<FamilyReset>())
            .add_system(Self::visibility_enable_system.run_in_state(GameState::FamilyEditor))
            .add_system_to_stage(
                CoreStage::PostUpdate,
                Self::visibility_disable_system.run_in_state(GameState::FamilyEditor),
            );
    }
}

impl FamilyEditorPlugin {
    fn spawn_system(mut commands: Commands, settings: Res<Settings>) {
        commands
            .spawn(EditableFamilyBundle::default())
            .with_children(|parent| {
                parent.spawn(PointLightBundle {
                    point_light: PointLight {
                        intensity: 1500.0,
                        shadows_enabled: true,
                        shadow_depth_bias: 0.25,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(4.0, 8.0, 4.0),
                    ..Default::default()
                });
                parent.spawn(OrbitCameraBundle::new(settings.video.render_graph_name()));
            });
    }

    fn visibility_enable_system(mut new_active_dolls: Query<&mut Visibility, Added<ActiveDoll>>) {
        for mut visibility in &mut new_active_dolls {
            visibility.is_visible = true;
        }
    }

    fn visibility_disable_system(
        removed_active_dolls: RemovedComponents<ActiveDoll>,
        mut visibility: Query<&mut Visibility>,
    ) {
        for entity in removed_active_dolls.iter() {
            // Entity could be despawned before.
            if let Ok(mut visibility) = visibility.get_mut(entity) {
                visibility.is_visible = false;
            }
        }
    }

    fn reset_family_system(
        mut commands: Commands,
        editable_dolls: Query<Entity, With<EditableDoll>>,
    ) {
        for entity in &editable_dolls {
            commands.entity(entity).despawn_recursive();
        }
    }

    fn cleanup_system(mut commands: Commands, family_editors: Query<Entity, With<EditableFamily>>) {
        commands.entity(family_editors.single()).despawn_recursive();
    }
}

#[derive(Bundle)]
struct EditableFamilyBundle {
    name: Name,
    family_editor: EditableFamily,

    #[bundle]
    spatial_bundle: SpatialBundle,
}

impl Default for EditableFamilyBundle {
    fn default() -> Self {
        Self {
            name: Name::new("New family"),
            family_editor: EditableFamily,
            spatial_bundle: Default::default(),
        }
    }
}

/// A root family editor component.
#[derive(Component, Default)]
pub(crate) struct EditableFamily;

#[derive(Bundle, Default)]
pub(crate) struct EditableDollBundle {
    editable_doll: EditableDoll,
    first_name: FirstName,
    last_name: LastName,
    transform: Transform,
}

/// Currently editing doll.
#[derive(Component, Default)]
pub(crate) struct EditableDoll;

/// An event on which family will be reset.
#[derive(Default)]
pub(crate) struct FamilyReset;
