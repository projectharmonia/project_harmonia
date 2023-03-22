use bevy::prelude::*;

use super::{
    actor::{ActiveActor, ActorBundle},
    family::{Actors, SelectedFamilySpawned},
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
        mut select_events: EventReader<SelectedFamilySpawned>,
        mut game_state: ResMut<NextState<GameState>>,
        actors: Query<&Actors>,
    ) {
        for event in select_events.iter() {
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
#[derive(Bundle, Default)]
pub(crate) struct EditableActorBundle {
    editable_actor: EditableActor,
    transform: Transform,

    #[bundle]
    actor_bundle: ActorBundle,
}

#[derive(Component, Default)]
pub(crate) struct EditableActor;

#[derive(Component)]
pub(crate) struct SelectedActor;

/// An event on which family will be reset.
#[derive(Default)]
pub(crate) struct FamilyReset;
