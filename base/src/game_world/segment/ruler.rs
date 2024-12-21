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
use itertools::MinMaxResult;

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
        segments: Query<(Ref<Segment>, &SegmentConnections, &Ruler)>,
        cameras: Query<&Transform, With<PlayerCamera>>,
        mut text: Query<(&mut Transform, &mut Text), Without<PlayerCamera>>,
    ) {
        for (segment, connections, &ruler) in &segments {
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
            );
        }
    }
}

fn draw_len(
    ruler_gizmos: &mut Gizmos<RulerConfig>,
    text: &mut Query<(&mut Transform, &mut Text), Without<PlayerCamera>>,
    segment: &Ref<Segment>,
    ruler: Ruler,
    segment_disp: Vec2,
    camera_pos: Vec2,
) {
    let camera_disp = camera_pos - segment.start;

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

    // Place on the center of the segment.
    let middle = segment.start.lerp(segment.end, 0.5);
    let text_offset = sign * 4.0 * Vec3::new(offset.x, 0.0, offset.y);
    text_transform.translation = Vec3::new(middle.x, 0.0, middle.y) + text_offset;

    // Rotate perpendicular to the segment and select the side closest to the camera.
    let mut angle = segment.displacement().angle_between(Vec2::X);
    if sign.is_sign_positive() {
        angle += PI;
    }
    text_transform.rotation = Quat::from_euler(EulerRot::YXZ, angle, FRAC_PI_2, 0.0);

    let text = &mut text.sections[0].value;
    text.clear();
    write!(text, "{:.2} m", segment.len()).unwrap();
}

fn draw_angle(
    angle_gizmos: &mut Gizmos<AngleConfig>,
    text: &mut Query<(&mut Transform, &mut Text), Without<PlayerCamera>>,
    segment: &Ref<Segment>,
    connections: &SegmentConnections,
    ruler: Ruler,
    segment_disp: Vec2,
    camera_rotation: Quat,
) {
    for (angle_entity, point_kind) in ruler
        .angle_entities
        .into_iter()
        .zip([PointKind::Start, PointKind::End])
    {
        let point = segment.point(point_kind);
        let point_disp = match point_kind {
            PointKind::Start => segment_disp,
            PointKind::End => -segment_disp,
        };

        let angle = match connections.side_angles(point_disp, point_kind) {
            MinMaxResult::NoElements => {
                if segment.is_changed() {
                    // Remove the text in case the segment is still exists, but don't have any angles.
                    let (_, mut text) = text.get_mut(angle_entity).unwrap();
                    let text = &mut text.sections[0].value;
                    text.clear();
                }
                continue;
            }
            MinMaxResult::OneElement(angle) => angle,
            MinMaxResult::MinMax(angle1, angle2) => {
                if angle1.abs() < angle2.abs() {
                    angle1
                } else {
                    angle2
                }
            }
        };

        let start_angle = point_disp.angle_between(Vec2::X);
        angle_gizmos.arc_3d(
            angle,
            1.0,
            Vec3::new(point.x, 0.0, point.y),
            Quat::from_rotation_y(start_angle),
            WHITE,
        );

        if !segment.is_changed() {
            // Update text only if redraw is required.
            continue;
        }

        let (mut text_transform, mut text) = text.get_mut(angle_entity).unwrap();

        // Place on the arc center.
        let text_offset = Quat::from_rotation_y(start_angle + angle / 2.0) * (1.5 * Vec3::X);
        text_transform.translation = Vec3::new(point.x, 0.0, point.y) + text_offset;

        // Rotate towards camera.
        let (y, ..) = camera_rotation.to_euler(EulerRot::YXZ);
        text_transform.rotation = Quat::from_euler(EulerRot::YXZ, y, FRAC_PI_2, PI);

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
