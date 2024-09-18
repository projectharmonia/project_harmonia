pub(crate) mod dynamic_mesh;

use std::{f32::consts::PI, mem};

use bevy::{
    ecs::{
        component::{ComponentHooks, ComponentId, StorageType},
        world::DeferredWorld,
    },
    prelude::*,
};
use bevy_replicon::prelude::*;
use itertools::{Itertools, MinMaxResult};
use serde::{Deserialize, Serialize};

use crate::{core::GameState, math::segment::Segment};

pub(super) struct SplinePlugin;

impl Plugin for SplinePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<SplineSegment>()
            .replicate::<SplineSegment>()
            .observe(Self::cleanup_connections)
            .add_systems(
                PostUpdate,
                Self::update_connections.run_if(in_state(GameState::InGame)),
            );
    }
}

impl SplinePlugin {
    /// Updates [`SplineConnections`] between segments.
    pub(super) fn update_connections(
        mut segments: Query<(Entity, &Visibility, &SplineSegment, &mut SplineConnections)>,
        children: Query<&Children>,
        changed_segments: Query<
            (Entity, &Parent, &Visibility, &SplineSegment),
            (
                Or<(Changed<SplineSegment>, Changed<Visibility>)>,
                With<SplineConnections>,
            ),
        >,
    ) {
        for (segment_entity, parent, visibility, &segment) in &changed_segments {
            // Take changed connections to avoid mutability issues.
            let (.., mut connections) = segments
                .get_mut(segment_entity)
                .expect("query is a subset of the changed query");
            let mut taken_connections = mem::take(&mut *connections);

            // Cleanup old connections.
            for connection in taken_connections.0.drain(..) {
                let (.., mut other_connections) = segments
                    .get_mut(connection.entity)
                    .expect("connected segment should also have connections");
                if let Some(index) = other_connections.position(segment_entity) {
                    other_connections.0.remove(index);
                }
            }

            // If segment have zero length or hidden, exclude it from connections.
            if segment.start != segment.end && visibility != Visibility::Hidden {
                // Scan all segments from this lot for possible connections.
                let mut iter = segments.iter_many_mut(children.get(**parent).unwrap());
                while let Some((other_entity, visibility, &other_segment, mut other_connections)) =
                    iter.fetch_next()
                {
                    if visibility == Visibility::Hidden || segment_entity == other_entity {
                        // Don't connect to hidden segments or self.
                        continue;
                    }

                    let kind = if segment.start == other_segment.start {
                        (PointKind::Start, PointKind::Start)
                    } else if segment.start == other_segment.end {
                        (PointKind::Start, PointKind::End)
                    } else if segment.end == other_segment.end {
                        (PointKind::End, PointKind::End)
                    } else if segment.end == other_segment.start {
                        (PointKind::End, PointKind::Start)
                    } else {
                        continue;
                    };

                    trace!(
                        "connecting segments `{segment_entity}` and `{other_entity}` as `{kind:?}`"
                    );
                    taken_connections.0.push(SplineConnection {
                        entity: other_entity,
                        segment: *other_segment,
                        kind,
                    });
                    other_connections.0.push(SplineConnection {
                        entity: segment_entity,
                        segment: *segment,
                        kind: (kind.1, kind.0),
                    });
                }
            }

            // Reinsert updated connections back.
            let (.., mut connections) = segments.get_mut(segment_entity).unwrap();
            *connections = taken_connections;
        }
    }

    fn cleanup_connections(
        trigger: Trigger<OnRemove, SplineSegment>,
        mut entities_buffer: Local<Vec<Entity>>,
        mut segments: Query<&mut SplineConnections>,
    ) {
        let connections = segments.get(trigger.entity()).unwrap();
        entities_buffer.extend(connections.iter().map(|connection| connection.entity));

        debug!("removing connections for segment `{}`", trigger.entity());
        for entity in entities_buffer.drain(..) {
            if let Ok(mut connections) = segments.get_mut(entity) {
                let index = connections
                    .position(trigger.entity())
                    .expect("segment connection should be done both ways");

                connections.0.remove(index);
            }
        }
    }
}

#[derive(Clone, Deref, DerefMut, Copy, Default, Deserialize, Reflect, Serialize)]
#[reflect(Component)]
pub(crate) struct SplineSegment(pub(super) Segment);

impl SplineSegment {
    fn on_insert(mut world: DeferredWorld, entity: Entity, _component_id: ComponentId) {
        world
            .commands()
            .entity(entity)
            .insert(SplineConnections::default());
    }
}

impl Component for SplineSegment {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(Self::on_insert);
    }
}

/// Dynamically updated component with precalculated connected entities for each segment point.
#[derive(Component, Default, Deref)]
pub(crate) struct SplineConnections(Vec<SplineConnection>);

impl SplineConnections {
    /// Returns the segments with the maximum and minimum angle relative
    /// to the displacement vector.
    pub(super) fn minmax_angles(&self, disp: Vec2, point_kind: PointKind) -> MinMaxResult<Segment> {
        self.0
            .iter()
            .filter(|connection| connection.kind.0 == point_kind)
            .map(|connection| {
                // Rotate points based on connection type.
                match connection.kind {
                    (PointKind::Start, PointKind::End) => connection.segment.inverse(),
                    (PointKind::End, PointKind::Start) => connection.segment,
                    (PointKind::Start, PointKind::Start) => connection.segment,
                    (PointKind::End, PointKind::End) => connection.segment.inverse(),
                }
            })
            .minmax_by_key(|segment| {
                let angle = segment.displacement().angle_between(disp);
                if angle < 0.0 {
                    angle + 2.0 * PI
                } else {
                    angle
                }
            })
    }

    fn position(&self, segment_entity: Entity) -> Option<usize> {
        self.iter()
            .position(|&SplineConnection { entity, .. }| entity == segment_entity)
    }
}

pub(crate) struct SplineConnection {
    entity: Entity,
    segment: Segment,
    kind: (PointKind, PointKind),
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub enum PointKind {
    Start,
    End,
}
