use avian3d::prelude::*;
use bevy::{
    ecs::{component::ComponentId, world::DeferredWorld},
    prelude::*,
};

use super::placing_object::PlacingObject;
use crate::{
    core::GameState,
    game_world::{
        family::building::wall::{self, Aperture, Apertures},
        segment::Segment,
        Layer,
    },
};

pub(super) struct WallMountPlugin;

impl Plugin for WallMountPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<WallMount>()
            .add_observer(init)
            .add_systems(
                PostUpdate,
                update_apertures
                    .before(wall::update_meshes)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

fn init(trigger: Trigger<OnAdd, WallMount>, mut objects: Query<&mut CollisionLayers>) {
    let Ok(mut collision_layers) = objects.get_mut(trigger.entity()) else {
        error!("wall mounts should always have collision");
        return;
    };

    collision_layers.filters.remove(Layer::Wall);
}

/// Updates [`Apertures`] based on spawned objects.
fn update_apertures(
    mut walls: Query<(Entity, &Parent, &Segment, &mut Apertures)>,
    mut objects: Query<
        (
            Entity,
            &Parent,
            &Visibility,
            &Transform,
            &WallMount,
            &mut ObjectWall,
            Has<PlacingObject>,
        ),
        Or<(Changed<Transform>, Changed<Visibility>)>,
    >,
) {
    for (
        object_entity,
        object_parent,
        visibility,
        transform,
        wall_mount,
        mut object_wall,
        placing_object,
    ) in &mut objects
    {
        if visibility == Visibility::Hidden {
            if let Some(wall_entity) = object_wall.0.take() {
                trace!("removing hidden `{object_entity}` from the aperture of `{wall_entity}`");
                let (.., mut apertures) = walls.get_mut(wall_entity).unwrap();
                apertures.remove(object_entity);
            }
            continue;
        }

        let translation = transform.translation;
        if let Some((wall_entity, _, segment, mut apertures)) = walls
            .iter_mut()
            .filter(|&(_, parent, ..)| parent == object_parent)
            .find(|(.., segment, _)| segment.contains(translation.xz()))
        {
            let distance = translation.xz().distance(segment.start);
            if let Some(current_entity) = object_wall.0 {
                if current_entity == wall_entity {
                    trace!("updating aperture of `{wall_entity}` for `{object_entity}`");
                    // Remove to update distance.
                    let mut aperture = apertures.remove(object_entity);

                    aperture.distance = distance;

                    apertures.insert(aperture);
                } else {
                    trace!("adding `{object_entity}` to the aperture of `{wall_entity}`");
                    apertures.insert(Aperture {
                        object_entity,
                        distance,
                        cutout: wall_mount.cutout.clone(),
                        hole: wall_mount.hole,
                        placing_object,
                    });

                    object_wall.0 = Some(wall_entity);

                    trace!("removing `{object_entity}` from the aperture of `{current_entity}`");
                    let (.., mut current_apertures) = walls.get_mut(current_entity).unwrap();
                    current_apertures.remove(object_entity);
                }
            } else {
                trace!("adding `{object_entity}` to the aperture of `{wall_entity}`");
                apertures.insert(Aperture {
                    object_entity,
                    distance,
                    cutout: wall_mount.cutout.clone(),
                    hole: wall_mount.hole,
                    placing_object,
                });

                object_wall.0 = Some(wall_entity);
            }
        } else if let Some(wall_entity) = object_wall.0.take() {
            trace!("removing `{object_entity}` from the aperture of `{wall_entity}`");
            let (.., mut apertures) = walls.get_mut(wall_entity).unwrap();
            apertures.remove(object_entity);
        }
    }
}

/// A component that marks that entity can be placed only on walls or inside them.
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ObjectWall)]
pub(crate) struct WallMount {
    /// Points for an aperture in the wall.
    ///
    /// Should be set clockwise if the object creates a hole (such as a window),
    /// or counterclockwise if it creates a clipping (such as a door).
    cutout: Vec<Vec2>,

    /// Should be set to `true` if the object creates a hole (such as a window).
    hole: bool,
}

#[derive(Default, Component)]
#[component(on_remove = Self::on_remove)]
struct ObjectWall(Option<Entity>);

impl ObjectWall {
    fn on_remove(mut world: DeferredWorld, entity: Entity, _component_id: ComponentId) {
        let object_wall = world.get::<Self>(entity).unwrap();
        if let Some(wall_entity) = object_wall.0 {
            if let Some(mut apertures) = world.get_mut::<Apertures>(wall_entity) {
                apertures.remove(entity);
            }
        }
    }
}
