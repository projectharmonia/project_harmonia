use bevy::prelude::*;
use iyes_loopless::prelude::*;

use crate::core::{
    action::{self, Action},
    city::ActiveCity,
    family::{BuildingMode, FamilyMode},
    game_state::GameState,
    ground::GroundPlugin,
};

use super::WallVertices;

pub(super) struct CreatingWallPlugin;

impl Plugin for CreatingWallPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            GroundPlugin::cursor_to_ground_system
                .pipe(Self::spawn_system)
                .run_if(action::just_pressed(Action::Confirm))
                .run_if_not(creating_active)
                .run_in_state(GameState::Family)
                .run_in_state(FamilyMode::Building)
                .run_in_state(BuildingMode::Walls),
        );
    }
}

impl CreatingWallPlugin {
    // fn test_system(
    //     mut commands: Commands,
    //     mut meshes: ResMut<Assets<Mesh>>,
    //     mut materials: ResMut<Assets<StandardMaterial>>,
    // ) {
    //     commands.spawn((
    //         WallVertices(vec![
    //             // (Vec2::ZERO, Vec2::X),
    //             // (Vec2::ZERO, Vec2::NEG_X),
    //             // (Vec2::ZERO, Vec2::Y),
    //             // (Vec2::ZERO, Vec2::NEG_Y),
    //             // (Vec2::ZERO, -Vec2::X * 4.0),
    //             (Vec2::ZERO, Vec2::X),
    //             (Vec2::ZERO, Vec2::NEG_X),
    //             (Vec2::ZERO, Vec2::Y),
    //             (Vec2::ZERO, Vec2::NEG_Y),
    //             // (Vec2::ZERO, Vec2::Y * 4.0),
    //             // (Vec2::ZERO, -Vec2::Y * 4.0),
    //             // (Vec2::Y * 4.0, -Vec2::ONE * 8.0),
    //             // (Vec2::Y * 4.0, -Vec2::ONE * 8.0),
    //         ]),
    //         PbrBundle {
    //             mesh: meshes.add(Mesh::new(PrimitiveTopology::TriangleList)),
    //             material: materials.add(StandardMaterial::default()),
    //             transform: Transform::from_xyz(1.0, 0.0, 0.0),
    //             ..Default::default()
    //         },
    //     ));
    // }

    fn spawn_system(
        In(position): In<Option<Vec2>>,
        mut commands: Commands,
        active_cities: Query<Entity, With<ActiveCity>>,
    ) {
        if let Some(position) = position {
            commands
                .entity(active_cities.single())
                .with_children(|parent| {
                    parent.spawn((
                        WallVertices(vec![(position, position + 10.0)]),
                        CreatingWall,
                    ));
                });
        }
    }
}

pub(crate) fn creating_active(building_walls: Query<(), With<CreatingWall>>) -> bool {
    !building_walls.is_empty()
}

#[derive(Component, Default)]
pub(crate) struct CreatingWall;
