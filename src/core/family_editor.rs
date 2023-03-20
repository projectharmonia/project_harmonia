use bevy::prelude::*;

use super::{
    doll::{ActiveDoll, DollBundle},
    family::{Dolls, SelectedFamilySpawned},
    game_state::GameState,
    player_camera::PlayerCameraBundle,
};

pub(super) struct FamilyEditorPlugin;

impl Plugin for FamilyEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FamilyReset>()
            .add_systems(
                (
                    Self::selection_system,
                    Self::visibility_enable_system,
                    Self::visibility_disable_system,
                    Self::reset_family_system.run_if(on_event::<FamilyReset>()),
                )
                    .in_set(OnUpdate(GameState::FamilyEditor)),
            )
            .add_systems(
                (Self::spawn_system, Self::cleanup_system)
                    .in_schedule(OnEnter(GameState::FamilyEditor)),
            );
    }
}

impl FamilyEditorPlugin {
    fn spawn_system(mut commands: Commands) {
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
                parent.spawn(PlayerCameraBundle::default());
            });
    }

    fn visibility_enable_system(
        mut new_selected_dolls: Query<&mut Visibility, Added<SelectedDoll>>,
    ) {
        for mut visibility in &mut new_selected_dolls {
            *visibility = Visibility::Visible;
        }
    }

    fn visibility_disable_system(
        mut removed_selected_dolls: RemovedComponents<SelectedDoll>,
        mut visibility: Query<&mut Visibility>,
    ) {
        for entity in &mut removed_selected_dolls {
            // Entity could be despawned before.
            if let Ok(mut visibility) = visibility.get_mut(entity) {
                *visibility = Visibility::Hidden;
            }
        }
    }

    fn selection_system(
        mut commands: Commands,
        mut select_events: EventReader<SelectedFamilySpawned>,
        mut game_state: ResMut<NextState<GameState>>,
        dolls: Query<&Dolls>,
    ) {
        for event in select_events.iter() {
            let dolls = dolls
                .get(event.0)
                .expect("spawned family should have dolls");
            let doll_entity = *dolls
                .first()
                .expect("family should always have at least one member");
            commands.entity(doll_entity).insert(ActiveDoll);
            game_state.set(GameState::Family);
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
    editable_family: EditableFamily,

    #[bundle]
    spatial_bundle: SpatialBundle,
}

impl Default for EditableFamilyBundle {
    fn default() -> Self {
        Self {
            name: Name::new("New family"),
            editable_family: EditableFamily,
            spatial_bundle: Default::default(),
        }
    }
}

/// A root family editor component.
#[derive(Component, Default)]
pub(crate) struct EditableFamily;

/// Components for a doll inside the editor.
#[derive(Bundle, Default)]
pub(crate) struct EditableDollBundle {
    editable_doll: EditableDoll,
    transform: Transform,

    #[bundle]
    doll_bundle: DollBundle,
}

#[derive(Component, Default)]
pub(crate) struct EditableDoll;

#[derive(Component)]
pub(crate) struct SelectedDoll;

/// An event on which family will be reset.
#[derive(Default)]
pub(crate) struct FamilyReset;
