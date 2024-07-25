use std::f32::consts::PI;

use bevy::prelude::*;

use crate::game_world::{
    actor::{human::Human, FirstName, LastName, SelectedActor, Sex},
    family::{FamilyMembers, SelectedFamilyCreated},
    player_camera::PlayerCameraBundle,
    WorldState,
};

pub(crate) struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<FamilyReset>()
            .add_systems(OnEnter(WorldState::FamilyEditor), Self::setup)
            .add_systems(
                Update,
                Self::play.run_if(in_state(WorldState::FamilyEditor)),
            )
            .add_systems(
                PostUpdate,
                Self::reset_family.run_if(on_event::<FamilyReset>()),
            );
    }
}

impl EditorPlugin {
    fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        debug!("initializing editor");
        commands
            .spawn((
                StateScoped(WorldState::FamilyEditor),
                EditableFamilyBundle::default(),
            ))
            .with_children(|parent| {
                parent.spawn(DirectionalLightBundle {
                    directional_light: DirectionalLight {
                        shadows_enabled: true,
                        ..Default::default()
                    },
                    transform: Transform::from_xyz(4.0, 7.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
                    ..Default::default()
                });
                parent.spawn(PlayerCameraBundle::new(&asset_server));
                parent.spawn(EditableActorBundle::default());
            });
    }

    fn play(
        mut commands: Commands,
        mut spawn_select_events: EventReader<SelectedFamilyCreated>,
        mut world_state: ResMut<NextState<WorldState>>,
        families: Query<&FamilyMembers>,
    ) {
        for members in families.iter_many(spawn_select_events.read().map(|event| event.0)) {
            info!("starting playing");
            let actor_entity = *members
                .first()
                .expect("family should always have at least one member");
            commands.entity(actor_entity).insert(SelectedActor);
            world_state.set(WorldState::Family);
        }
    }

    fn reset_family(
        mut commands: Commands,
        actors: Query<Entity, With<EditableActor>>,
        families: Query<Entity, With<EditableFamily>>,
    ) {
        info!("resetting family");
        for entity in &actors {
            commands.entity(entity).despawn_recursive();
        }

        // Spawn a new actor for editing.
        commands.entity(families.single()).with_children(|parent| {
            parent.spawn(EditableActorBundle::default());
        });
    }
}

#[derive(Bundle)]
struct EditableFamilyBundle {
    editable_family: EditableFamily,
    spatial_bundle: SpatialBundle,
}

impl Default for EditableFamilyBundle {
    fn default() -> Self {
        Self {
            editable_family: EditableFamily,
            spatial_bundle: Default::default(),
        }
    }
}

/// A root family editor component.
#[derive(Component, Default)]
pub struct EditableFamily;

/// Components for a actor inside the editor.
#[derive(Bundle)]
pub struct EditableActorBundle {
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
pub struct EditableActor;

/// Event that resets currently editing family.
#[derive(Default, Event)]
pub struct FamilyReset;
