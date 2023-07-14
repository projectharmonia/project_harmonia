use std::f32::consts::PI;

use bevy::prelude::*;

use crate::core::{
    actor::{race::human::Human, ActiveActor, FirstName, LastName, Sex},
    family::{FamilyMembers, SelectedFamilySpawned},
    game_state::GameState,
    player_camera::PlayerCameraBundle,
};

pub(super) struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FamilyReset>()
            .add_systems(
                (
                    Self::selection_system,
                    Self::reset_family_system.run_if(on_event::<FamilyReset>()),
                )
                    .in_set(OnUpdate(GameState::FamilyEditor)),
            )
            .add_system(Self::setup_system.in_schedule(OnEnter(GameState::FamilyEditor)))
            .add_system(Self::cleanup_system.in_schedule(OnExit(GameState::FamilyEditor)));
    }
}

impl EditorPlugin {
    fn setup_system(mut commands: Commands) {
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
            })
            .with_children(|parent| {
                parent.spawn(EditableActorBundle::default());
            });
    }

    fn selection_system(
        mut commands: Commands,
        mut spawn_select_events: EventReader<SelectedFamilySpawned>,
        mut game_state: ResMut<NextState<GameState>>,
        families: Query<&FamilyMembers>,
    ) {
        for event in &mut spawn_select_events {
            let members = families
                .get(event.0)
                .expect("spawned family should have actors");
            let actor_entity = *members
                .first()
                .expect("family should always have at least one member");
            commands.entity(actor_entity).insert(ActiveActor);
            game_state.set(GameState::Family);
        }
    }

    fn reset_family_system(mut commands: Commands, actors: Query<Entity, With<EditableActor>>) {
        for entity in &actors {
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
                visibility: Visibility::Hidden,
                ..Default::default()
            },
        }
    }
}

#[derive(Component, Default)]
pub(crate) struct EditableActor;

/// Event that resets currently editing family.
#[derive(Default)]
pub(crate) struct FamilyReset;
