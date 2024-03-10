use bevy::{math::Vec3Swizzles, prelude::*, window::PrimaryWindow};
use bevy_replicon::prelude::*;
use bevy_xpbd_3d::prelude::*;
use leafwing_input_manager::common_conditions::action_just_pressed;

use super::{Wall, WallCreate, WallCreateConfirmed};
use crate::core::{
    action::Action,
    cursor_hover::CursorHover,
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    lot::LotVertices,
    player_camera::PlayerCamera,
};

pub(super) struct SpawningWallPlugin;

impl Plugin for SpawningWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(FamilyMode::Building), Self::end_creating)
            .add_systems(OnExit(BuildingMode::Walls), Self::end_creating)
            .add_systems(
                PreUpdate,
                Self::end_creating
                    .after(ClientSet::Receive)
                    .run_if(in_state(GameState::Family))
                    .run_if(in_state(FamilyMode::Building))
                    .run_if(in_state(BuildingMode::Walls))
                    .run_if(any_with_component::<SpawningWall>)
                    .run_if(on_event::<WallCreateConfirmed>()),
            )
            .add_systems(
                Update,
                (
                    Self::start_creating
                        .run_if(action_just_pressed(Action::Confirm))
                        .run_if(not(any_with_component::<SpawningWall>)),
                    (
                        (
                            Self::update_end,
                            Self::update_material,
                            Self::confirm.run_if(action_just_pressed(Action::Confirm)),
                        )
                            .run_if(not(any_with_component::<UnconfirmedWall>)),
                        Self::end_creating.run_if(action_just_pressed(Action::Cancel)),
                    )
                        .run_if(any_with_component::<SpawningWall>),
                )
                    .run_if(in_state(GameState::Family))
                    .run_if(in_state(FamilyMode::Building))
                    .run_if(in_state(BuildingMode::Walls)),
            );
    }
}

const SNAP_DELTA: f32 = 0.5;

impl SpawningWallPlugin {
    fn start_creating(
        mut commands: Commands,
        walls: Query<&Wall>,
        lots: Query<(Entity, Option<&Children>, &LotVertices)>,
        hovered: Query<&CursorHover>,
    ) {
        if let Ok(position) = hovered.get_single().map(|hover| hover.xz()) {
            if let Some((entity, children, _)) = lots
                .iter()
                .find(|(.., vertices)| vertices.contains_point(position))
            {
                // Use an existing point if it is within the `SNAP_DELTA` distance.
                let point = walls
                    .iter_many(children.into_iter().flatten())
                    .flat_map(|wall| [wall.start, wall.end])
                    .find(|vertex| vertex.distance(position) < SNAP_DELTA)
                    .unwrap_or(position);

                commands.entity(entity).with_children(|parent| {
                    parent.spawn((
                        SpawningWall,
                        Wall {
                            start: point,
                            end: point,
                        },
                    ));
                });
            }
        }
    }

    fn update_material(
        mut materials: ResMut<Assets<StandardMaterial>>,
        mut walls: Query<
            (&mut Handle<StandardMaterial>, &CollidingEntities),
            (Changed<CollidingEntities>, With<SpawningWall>),
        >,
    ) {
        for (mut material_handle, colliding_entities) in &mut walls {
            let mut material = materials
                .get(&*material_handle)
                .cloned()
                .expect("material handle should be valid");

            material.alpha_mode = AlphaMode::Add;
            material.base_color = if colliding_entities.is_empty() {
                Color::WHITE
            } else {
                Color::RED
            };
            *material_handle = materials.add(material);

            debug!("assigned material color for placing wall");
        }
    }

    fn update_end(
        mut spawning_walls: Query<(&mut Wall, &Parent), With<SpawningWall>>,
        walls: Query<&Wall, Without<SpawningWall>>,
        children: Query<&Children>,
        windows: Query<&Window, With<PrimaryWindow>>,
        cameras: Query<(&GlobalTransform, &Camera), With<PlayerCamera>>,
    ) {
        let Some(cursor_pos) = windows.single().cursor_position() else {
            return;
        };

        let (&camera_transform, camera) = cameras.single();
        let ray = camera
            .viewport_to_world(&camera_transform, cursor_pos)
            .expect("ray should be created from screen coordinates");

        let Some(distance) = ray.intersect_plane(Vec3::ZERO, Plane3d::new(Vec3::Y)) else {
            return;
        };

        let (mut wall, parent) = spawning_walls.single_mut();
        let children = children.get(**parent).unwrap();

        // Use an already existing vertex if it is within the `SNAP_DELTA` distance if one exists.
        let position = ray.get_point(distance).xz();
        let vertex = walls
            .iter_many(children)
            .flat_map(|wall| [wall.start, wall.end])
            .find(|vertex| vertex.distance(position) < SNAP_DELTA)
            .unwrap_or(position);

        wall.end = vertex;
    }

    fn confirm(
        mut commands: Commands,
        mut create_events: EventWriter<WallCreate>,
        mut walls: Query<(Entity, &Parent, &Wall), With<SpawningWall>>,
    ) {
        let (wall_entity, parent, &wall) = walls.single_mut();

        commands.entity(wall_entity).insert(UnconfirmedWall);

        create_events.send(WallCreate {
            lot_entity: **parent,
            wall,
        });
    }

    fn end_creating(mut commands: Commands, walls: Query<Entity, With<SpawningWall>>) {
        commands.entity(walls.single()).despawn();
    }
}

#[derive(Component, Default)]
pub(crate) struct SpawningWall;

#[derive(Component, Default)]
pub(crate) struct UnconfirmedWall;
