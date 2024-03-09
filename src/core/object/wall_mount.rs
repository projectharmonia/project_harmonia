use bevy::{prelude::*, transform::TransformSystem};
use bevy_xpbd_3d::{math::PI, prelude::*};

use super::{ObjectComponent, ReflectObjectComponent};
use crate::core::{
    city::CityMode,
    family::FamilyMode,
    game_state::GameState,
    game_world::WorldName,
    object::placing_object::{PlacingObject, PlacingObjectPlugin},
    wall::{wall_mesh::HALF_WIDTH, Aperture, Apertures, Wall, WallPlugin},
    Layer,
};

pub(super) struct WallMountPlugin;

impl Plugin for WallMountPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Vec2>()
            .register_type::<Vec<Vec2>>()
            .register_type::<WallMount>()
            .add_systems(
                Update,
                (
                    Self::init.run_if(resource_exists::<WorldName>),
                    (
                        Self::init_placing.before(PlacingObjectPlugin::rotate),
                        Self::snap
                            .before(PlacingObjectPlugin::check_collision)
                            .after(PlacingObjectPlugin::apply_transform),
                    )
                        .run_if(
                            in_state(GameState::City)
                                .and_then(in_state(CityMode::Objects))
                                .or_else(
                                    in_state(GameState::Family)
                                        .and_then(in_state(FamilyMode::Building)),
                                ),
                        ),
                ),
            )
            .add_systems(
                PostUpdate,
                (Self::update_apertures, Self::cleanup_apertures)
                    .before(WallPlugin::update_meshes)
                    .after(TransformSystem::TransformPropagate)
                    .run_if(resource_exists::<WorldName>),
            );
    }
}

impl WallMountPlugin {
    /// Additional intializaiton for wall mount objects.
    fn init(
        mut commands: Commands,
        mut objects: Query<(Entity, &mut CollisionLayers), Added<WallMount>>,
    ) {
        for (entity, mut collision_layers) in &mut objects {
            collision_layers.filters.remove(Layer::Wall);
            commands.entity(entity).insert(ObjectWall::default());
        }
    }

    /// Additional intializaiton for placing wall mount objects.
    fn init_placing(mut placing_objects: Query<&mut PlacingObject, Added<WallMount>>) {
        if let Ok(mut placing_object) = placing_objects.get_single_mut() {
            placing_object.rotation_step = PI;
        }
    }

    fn snap(
        walls: Query<&Wall>,
        mut placing_objects: Query<(&mut Transform, &mut PlacingObject, &WallMount)>,
    ) {
        let Ok((mut transform, mut placing_object, wall_mount)) = placing_objects.get_single_mut()
        else {
            return;
        };

        const SNAP_DELTA: f32 = 1.0;
        let translation_2d = transform.translation.xz();
        if let Some((wall, wall_point)) = walls
            .iter()
            .map(|wall| (wall, wall.closest_point(translation_2d)))
            .find(|(_, point)| point.distance(translation_2d) <= SNAP_DELTA)
        {
            const GAP: f32 = 0.03; // A small gap between the object and wall to avoid collision.
            let disp = wall.displacement();
            let sign = disp.perp_dot(translation_2d - wall_point).signum();
            let offset = match wall_mount {
                WallMount::Embed { .. } => Vec2::ZERO,
                WallMount::Attach => sign * disp.perp().normalize() * (HALF_WIDTH + GAP),
            };
            let snap_point = wall_point + offset;
            let angle = disp.angle_between(Vec2::X * sign);
            transform.translation.x = snap_point.x;
            transform.translation.z = snap_point.y;
            if !placing_object.allowed_place {
                // Apply rotation only for newly snapped objects.
                transform.rotation = Quat::from_rotation_y(angle);
                placing_object.allowed_place = true;
            }
        } else if placing_object.allowed_place {
            placing_object.allowed_place = false;
        }
    }

    fn update_apertures(
        mut walls: Query<(Entity, &mut Apertures, &Wall)>,
        mut wall_mounts: Query<
            (Entity, &GlobalTransform, &WallMount, &mut ObjectWall),
            Changed<GlobalTransform>,
        >,
    ) {
        for (object_entity, transform, wall_mount, mut object_wall) in &mut wall_mounts {
            let WallMount::Embed { cutout, hole } = wall_mount else {
                continue;
            };

            let translation = transform.translation();
            if let Some((wall_entity, mut apertures, wall)) = walls
                .iter_mut()
                .find(|(.., wall)| wall.contains(translation.xz()))
            {
                let distance = translation.xz().distance(wall.start);
                if let Some(current_entity) = object_wall.0 {
                    if current_entity == wall_entity {
                        apertures.update_translation(object_entity, translation, distance)
                    } else {
                        apertures.insert(Aperture {
                            object_entity,
                            translation,
                            distance,
                            cutout: cutout.clone(),
                            hole: *hole,
                        });

                        object_wall.0 = Some(wall_entity);

                        let (_, mut current_apertures, _) = walls
                            .get_mut(current_entity)
                            .expect("all doors should have apertures");
                        let index = current_apertures
                            .position(object_entity)
                            .expect("entity should have been added before");
                        current_apertures.remove(index);
                    }
                } else {
                    apertures.insert(Aperture {
                        object_entity,
                        translation,
                        distance,
                        cutout: cutout.clone(),
                        hole: *hole,
                    });

                    object_wall.0 = Some(wall_entity);
                }
            } else if let Some(wall_entity) = object_wall.0.take() {
                let (_, mut current_apertures, _) = walls
                    .get_mut(wall_entity)
                    .expect("all doors should have apertures");
                let index = current_apertures
                    .position(object_entity)
                    .expect("entity should have been added before");
                current_apertures.remove(index);
            }
        }
    }

    fn cleanup_apertures(
        mut removed_objects: RemovedComponents<ObjectWall>,
        mut walls: Query<&mut Apertures>,
    ) {
        for entity in removed_objects.read() {
            for mut apertures in &mut walls {
                if let Some(index) = apertures.position(entity) {
                    apertures.remove(index);
                }
            }
        }
    }
}

/// A component that marks that entity can be placed only on walls or inside them.
#[derive(Component, Reflect)]
#[reflect(Component, ObjectComponent)]
pub(crate) enum WallMount {
    Attach,
    Embed {
        /// Points for an aperture in the wall.
        ///
        /// Should be set clockwise if the object requires a full opening (such as a window),
        /// or counterclockwise if it creates a half opening (such as a door).
        cutout: Vec<Vec2>,

        /// Should be set to `true` if the object requires a full opening (such as a window).
        hole: bool,
    },
}

// To implement `Reflect`.
impl FromWorld for WallMount {
    fn from_world(_world: &mut World) -> Self {
        Self::Attach
    }
}

impl ObjectComponent for WallMount {
    fn insert_on_spawning(&self) -> bool {
        true
    }

    fn insert_on_placing(&self) -> bool {
        true
    }
}

#[derive(Component, Default)]
struct ObjectWall(Option<Entity>);
