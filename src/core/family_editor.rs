use std::f32::consts::PI;

use bevy::prelude::*;

use super::{
    actor::{race::human::Human, ActiveActor, FirstName, LastName, Sex},
    family::{FamilyActors, SelectedFamilySpawned},
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
            .add_system(Self::spawn_system.in_schedule(OnEnter(GameState::FamilyEditor)))
            .add_system(Self::cleanup_system.in_schedule(OnExit(GameState::FamilyEditor)));
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

    fn visibility_enable_system(mut selected_actors: Query<&mut Visibility, Added<SelectedActor>>) {
        for mut visibility in &mut selected_actors {
            *visibility = Visibility::Visible;
        }
    }

    fn visibility_disable_system(
        mut deselected_actors: RemovedComponents<SelectedActor>,
        mut visibility: Query<&mut Visibility>,
    ) {
        for entity in &mut deselected_actors {
            // Entity could be despawned before.
            if let Ok(mut visibility) = visibility.get_mut(entity) {
                *visibility = Visibility::Hidden;
            }
        }
    }

    fn selection_system(
        mut commands: Commands,
        mut spawn_select_events: EventReader<SelectedFamilySpawned>,
        mut game_state: ResMut<NextState<GameState>>,
        actors: Query<&FamilyActors>,
    ) {
        for event in &mut spawn_select_events {
            let actors = actors
                .get(event.0)
                .expect("spawned family should have actors");
            let actor_entity = *actors
                .first()
                .expect("family should always have at least one member");
            commands.entity(actor_entity).insert(ActiveActor);
            game_state.set(GameState::Family);
        }
    }

    fn reset_family_system(
        mut commands: Commands,
        editable_actors: Query<Entity, With<EditableActor>>,
    ) {
        for entity in &editable_actors {
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

/// Components for a actor inside the editor.
#[derive(Bundle)]
pub(crate) struct EditableActorBundle {
    human: Human, // TODO: Select race
    first_name: FirstName,
    last_name: LastName,
    sex: Sex,
    editable_actor: EditableActor,

    #[bundle]
    spatial_bundle: SpatialBundle,
}

impl Default for EditableActorBundle {
    fn default() -> Self {
        Self {
            human: Human,
            first_name: Default::default(),
            last_name: Default::default(),
            sex: Default::default(),
            editable_actor: EditableActor,
            spatial_bundle: SpatialBundle {
                transform: Transform::from_rotation(Quat::from_rotation_y(PI)), // Rotate towards camera.
                ..Default::default()
            },
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct EditableActor;

#[derive(Component)]
pub(crate) struct SelectedActor;

/// Event that resets currently editing family.
#[derive(Default)]
pub(crate) struct FamilyReset;
