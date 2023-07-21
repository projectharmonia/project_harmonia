use std::{f32::consts::PI, fs};

use anyhow::{Context, Result};
use bevy::prelude::*;

use crate::core::{
    actor::{race::human::Human, ActiveActor, FirstName, LastName, Sex},
    error,
    family::{FamilyMembers, SelectedFamilySpawned},
    game_paths::GamePaths,
    game_state::GameState,
    player_camera::PlayerCameraBundle,
};

use super::family_spawn::{FamilyScene, FamilySceneSerializer};

pub(crate) struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FamilyReset>()
            .add_systems(OnEnter(GameState::FamilyEditor), Self::setup_system)
            .add_systems(OnExit(GameState::FamilyEditor), Self::cleanup_system)
            .add_systems(
                Update,
                (
                    Self::selection_system,
                    Self::reset_family_system.run_if(on_event::<FamilyReset>()),
                    Self::scene_save_system.pipe(error::report),
                )
                    .run_if(in_state(GameState::FamilyEditor)),
            );
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

    pub(crate) fn scene_save_system(
        registry: Res<AppTypeRegistry>,
        game_paths: Res<GamePaths>,
        family_scenes: Query<&FamilyScene, Added<FamilyScene>>,
    ) -> Result<()> {
        if let Ok(family_scene) = family_scenes.get_single() {
            fs::create_dir_all(&game_paths.families)
                .with_context(|| format!("unable to create {:?}", game_paths.families))?;

            let registry = registry.read();
            let serializer = FamilySceneSerializer::new(family_scene, &registry);
            let ron = ron::to_string(&serializer).expect("unable to serialize family scene");
            let family_path = game_paths.family_path(&family_scene.name);
            fs::write(&family_path, ron)
                .with_context(|| format!("unable to save game to {family_path:?}"))?;
        }

        Ok(())
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
#[derive(Default, Event)]
pub(crate) struct FamilyReset;
