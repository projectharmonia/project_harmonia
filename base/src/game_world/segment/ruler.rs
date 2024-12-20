use std::{
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
use bevy_mod_billboard::{prelude::*, BillboardLockAxis};
use itertools::MinMaxResult;

use super::{PointKind, Segment, SplineConnections};
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
        segments: Query<(Ref<Segment>, &SplineConnections, &Ruler)>,
        cameras: Query<&Transform, With<PlayerCamera>>,
        mut text: Query<(&mut Transform, &mut Text), Without<PlayerCamera>>,
    ) {
        for (segment, connections, ruler) in &segments {
            let camera_transform = cameras.single();
            let camera_pos = camera_transform.translation.xz();

            let segment_disp = segment.displacement();
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

            for point_kind in [PointKind::Start, PointKind::End] {
                let point = segment.point(point_kind);
                let point_disp = match point_kind {
                    PointKind::Start => segment_disp,
                    PointKind::End => -segment_disp,
                };
                match connections.side_angles(point_disp, point_kind) {
                    MinMaxResult::NoElements => (),
                    MinMaxResult::OneElement(angle) => {
                        draw_angle(&mut angle_gizmos, angle, point, point_disp);
                    }
                    MinMaxResult::MinMax(angle1, angle2) => {
                        let angle = if angle1.abs() > angle2.abs() {
                            angle2
                        } else {
                            angle1
                        };
                        draw_angle(&mut angle_gizmos, angle, point, point_disp);
                    }
                }
            }

            if !segment.is_changed() {
                continue;
            }

            let (mut text_transform, mut text) = text.get_mut(ruler.text_entity).unwrap();

            let middle = segment.start.lerp(segment.end, 0.5);
            let text_offset = Vec3::new(offset.x, 0.0, offset.y) * sign * 4.0;
            text_transform.translation = Vec3::new(middle.x, 0.1, middle.y) + text_offset;

            let mut angle = segment.displacement().angle_between(Vec2::X);
            if sign.is_sign_positive() {
                angle += PI;
            }
            text_transform.rotation = Quat::from_euler(EulerRot::YXZ, angle, FRAC_PI_2, 0.0);

            let text = &mut text.sections[0].value;
            text.clear();
            write!(text, "{:.2} m", segment.len()).unwrap();
        }
    }
}

fn draw_angle(angle_gizmos: &mut Gizmos<AngleConfig>, angle: f32, point: Vec2, segment_disp: Vec2) {
    let start_angle = segment_disp.angle_between(Vec2::X);
    angle_gizmos.arc_3d(
        angle,
        1.0,
        Vec3::new(point.x, 0.0, point.y),
        Quat::from_rotation_y(start_angle),
        WHITE,
    );
}

#[derive(GizmoConfigGroup, Default, Reflect)]
struct RulerConfig;

#[derive(GizmoConfigGroup, Default, Reflect)]
struct AngleConfig;

pub(crate) struct Ruler {
    text_entity: Entity,
}

impl Ruler {
    fn on_insert(mut world: DeferredWorld, entity: Entity, _component_id: ComponentId) {
        let font = world.resource::<RulerFont>().0.clone();

        let text_entity = world
            .commands()
            .spawn((
                BillboardTextBundle {
                    transform: Transform::from_scale(Vec3::splat(0.005)),
                    text: Text::from_section(
                        "",
                        TextStyle {
                            font,
                            font_size: 100.0,
                            color: WHITE.into(),
                        },
                    ),
                    ..Default::default()
                },
                BillboardLockAxis {
                    rotation: true,
                    ..Default::default()
                },
            ))
            .id();

        let mut ruler = world.get_mut::<Self>(entity).unwrap();
        ruler.text_entity = text_entity;

        world.commands().entity(entity).add_child(text_entity);
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
            text_entity: Entity::PLACEHOLDER,
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
