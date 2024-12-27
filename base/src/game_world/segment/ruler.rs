use std::{
    array,
    f32::consts::{FRAC_PI_2, PI},
    fmt::Write,
};

use bevy::{
    color::palettes::css::WHITE,
    ecs::{
        component::{ComponentHooks, ComponentId, StorageType},
        world::DeferredWorld,
    },
    prelude::*,
};
use bevy_mod_billboard::{prelude::*, BillboardDepth, BillboardLockAxis};

use super::{PointKind, Segment, SegmentConnections};
use crate::game_world::{family::building::BuildingMode, player_camera::PlayerCamera};

pub(super) struct RulerPlugin;

impl Plugin for RulerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RulerFont>()
            .insert_gizmo_config(
                RulerConfig,
                GizmoConfig {
                    line_width: 100.0,
                    line_perspective: true,
                    line_style: GizmoLineStyle::Dotted,
                    depth_bias: -1.0,
                    ..Default::default()
                },
            )
            .insert_gizmo_config(
                AngleConfig,
                GizmoConfig {
                    line_width: 60.0,
                    line_perspective: true,
                    depth_bias: -1.0,
                    ..Default::default()
                },
            )
            .add_systems(Update, Self::draw.run_if(in_state(BuildingMode::Walls)));
    }
}

impl RulerPlugin {
    fn draw(
        mut ruler_gizmos: Gizmos<RulerConfig>,
        mut angle_gizmos: Gizmos<AngleConfig>,
        segments: Query<(Ref<Segment>, &SegmentConnections, &Ruler, &Transform)>,
        cameras: Query<&Transform, With<PlayerCamera>>,
        mut text: Query<(&mut Transform, &mut Text), (Without<PlayerCamera>, Without<Segment>)>,
    ) {
        for (segment, connections, &ruler, segment_transform) in &segments {
            if segment.is_zero() {
                continue;
            }

            let camera_transform = cameras.single();
            let segment_disp = segment.displacement();

            draw_len(
                &mut ruler_gizmos,
                &mut text,
                &segment,
                ruler,
                segment_disp,
                camera_transform.translation.xz(),
            );

            draw_angle(
                &mut angle_gizmos,
                &mut text,
                &segment,
                connections,
                ruler,
                segment_disp,
                camera_transform.rotation,
                segment_transform.rotation,
            );
        }
    }
}

fn draw_len(
    ruler_gizmos: &mut Gizmos<RulerConfig>,
    text: &mut Query<(&mut Transform, &mut Text), (Without<PlayerCamera>, Without<Segment>)>,
    segment: &Ref<Segment>,
    ruler: Ruler,
    segment_disp: Vec2,
    camera_transform: Vec2,
) {
    let camera_disp = camera_transform - segment.start;

    let offset = segment_disp.perp().normalize_or_zero() * 0.25;
    let sign = segment_disp.perp_dot(camera_disp).signum();

    let start = segment.start + offset * sign;
    let end = segment.end + offset * sign;

    ruler_gizmos.line(
        Vec3::new(start.x, 0.0, start.y),
        Vec3::new(end.x, 0.0, end.y),
        WHITE,
    );

    if !segment.is_changed() {
        // Update text only if redraw is required.
        return;
    }

    let (mut text_transform, mut text) = text.get_mut(ruler.len_entity).unwrap();

    // Place it at the center of the segment with an offset from the side closest to the camera.
    let text_pos = Vec3::X * segment.len() / 2.0;
    let text_offset = sign * Vec3::Z;
    text_transform.translation = text_pos + text_offset;

    // Rotate from the side closest to the camera.
    let angle = if sign.is_sign_positive() { PI } else { 0.0 };
    text_transform.rotation = Quat::from_euler(EulerRot::XYZ, FRAC_PI_2, 0.0, angle);

    let text = &mut text.sections[0].value;
    text.clear();
    write!(text, "{:.2} m", segment_disp.length()).unwrap();
}

fn draw_angle(
    angle_gizmos: &mut Gizmos<AngleConfig>,
    text: &mut Query<(&mut Transform, &mut Text), (Without<PlayerCamera>, Without<Segment>)>,
    segment: &Ref<Segment>,
    connections: &SegmentConnections,
    ruler: Ruler,
    segment_disp: Vec2,
    camera_rotation: Quat,
    segment_rotation: Quat,
) {
    for (angle_entity, point_kind) in ruler
        .angle_entities
        .into_iter()
        .zip([PointKind::Start, PointKind::End])
    {
        let point = segment.point(point_kind);
        let sign = match point_kind {
            PointKind::Start => 1.0,
            PointKind::End => -1.0,
        };

        let Some(angle) = connections.min_angle(point_kind, sign * segment_disp) else {
            if segment.is_changed() {
                // Remove the text in case the segment is still exists, but don't have any angles.
                let (_, mut text) = text.get_mut(angle_entity).unwrap();
                let text = &mut text.sections[0].value;
                text.clear();
            }
            continue;
        };

        angle_gizmos.arc_3d(
            angle,
            1.0,
            Vec3::new(point.x, 0.0, point.y),
            segment_rotation,
            WHITE,
        );

        if !segment.is_changed() {
            // Update text only if redraw is required.
            continue;
        }

        let (mut text_transform, mut text) = text.get_mut(angle_entity).unwrap();

        // Place on the arc center.
        let text_pos = match point_kind {
            PointKind::Start => Vec3::ZERO,
            PointKind::End => Vec3::X * segment.len(),
        };
        let mut text_angle = angle / 2.0;
        if point_kind == PointKind::End {
            text_angle += PI;
        }
        let text_offset = Quat::from_rotation_y(text_angle) * (1.5 * Vec3::X);
        text_transform.translation = text_pos + text_offset;

        // Rotate towards camera.
        let (yaw, ..) = camera_rotation.to_euler(EulerRot::YXZ);
        text_transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw + angle, FRAC_PI_2, 0.0);

        let text = &mut text.sections[0].value;
        text.clear();
        write!(text, "{:.0}°", angle.abs().to_degrees()).unwrap();
    }
}

#[derive(GizmoConfigGroup, Default, Reflect)]
struct RulerConfig;

#[derive(GizmoConfigGroup, Default, Reflect)]
struct AngleConfig;

#[derive(Clone, Copy)]
pub(crate) struct Ruler {
    len_entity: Entity,
    angle_entities: [Entity; 2],
}

impl Ruler {
    fn on_insert(mut world: DeferredWorld, entity: Entity, _component_id: ComponentId) {
        let font_handle = world.resource::<RulerFont>().0.clone();

        let len_entity = world
            .commands()
            .spawn((
                BillboardTextBundle {
                    transform: Transform::from_scale(Vec3::splat(0.005)),
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font: font_handle.clone(),
                            font_size: 100.0,
                            color: WHITE.into(),
                        },
                    ),
                    billboard_depth: BillboardDepth(false),
                    ..Default::default()
                },
                BillboardLockAxis {
                    rotation: true,
                    ..Default::default()
                },
            ))
            .id();

        let angle_entities = array::from_fn(|_| {
            world
                .commands()
                .spawn((
                    BillboardTextBundle {
                        transform: Transform::from_scale(Vec3::splat(0.005)),
                        text: Text::from_section(
                            "",
                            TextStyle {
                                font: font_handle.clone(),
                                font_size: 80.0,
                                color: WHITE.into(),
                            },
                        ),
                        billboard_depth: BillboardDepth(false),
                        ..Default::default()
                    },
                    BillboardLockAxis {
                        rotation: true,
                        ..Default::default()
                    },
                ))
                .id()
        });

        let mut ruler = world.get_mut::<Self>(entity).unwrap();
        ruler.len_entity = len_entity;
        ruler.angle_entities = angle_entities;

        world
            .commands()
            .entity(entity)
            .add_child(len_entity)
            .push_children(&angle_entities);
    }
}

impl Component for Ruler {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_insert(Self::on_insert);
    }
}

impl Default for Ruler {
    fn default() -> Self {
        Self {
            len_entity: Entity::PLACEHOLDER,
            angle_entities: [Entity::PLACEHOLDER; 2],
        }
    }
}

#[derive(Resource)]
struct RulerFont(Handle<Font>);

impl FromWorld for RulerFont {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.resource::<AssetServer>();
        let font_handle = asset_server.load("base/fonts/FiraMono-Bold.ttf");
        Self(font_handle)
    }
}
