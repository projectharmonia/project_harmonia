use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    asset::AssetPath, core_pipeline::experimental::taa::TemporalAntiAliasing,
    ecs::system::SystemParam, pbr::ScreenSpaceAmbientOcclusion, prelude::*,
};
use bevy_enhanced_input::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use crate::{
    asset::collection::{AssetCollection, Collection},
    common_conditions::in_any_state,
    game_world::WorldState,
    settings::Settings,
};

pub(super) struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Collection<EnvironmentMap>>()
            .add_input_context::<PlayerCamera>()
            .add_observer(Self::init)
            .add_observer(Self::pan)
            .add_observer(Self::zoom)
            .add_observer(Self::rotate)
            .add_systems(
                Update,
                Self::apply_transform.run_if(in_any_state([
                    WorldState::FamilyEditor,
                    WorldState::City,
                    WorldState::Family,
                ])),
            );
    }
}

impl PlayerCameraPlugin {
    fn init(
        trigger: Trigger<OnAdd, PlayerCamera>,
        mut cameras: Query<&mut EnvironmentMapLight>,
        environment_map: Res<Collection<EnvironmentMap>>,
    ) {
        debug!("initializing player camera");
        let mut env_light = cameras.get_mut(trigger.entity()).unwrap();
        env_light.diffuse_map = environment_map.handle(EnvironmentMap::Diffuse);
        env_light.specular_map = environment_map.handle(EnvironmentMap::Specular);
        env_light.intensity = 800.0;
    }

    fn pan(
        trigger: Trigger<Fired<PanCamera>>,
        world_state: Res<State<WorldState>>,
        camera: Single<(&mut OrbitOrigin, &Transform, &SpringArm)>,
    ) {
        if *world_state == WorldState::FamilyEditor {
            return;
        }

        // Calculate direction without camera's tilt.
        let (mut orbit_origin, transform, spring_arm) = camera.into_inner();
        let forward = transform.forward();
        let camera_dir = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, camera_dir);

        // Movement consists of X and -Z components, so swap Y and Z with negation.
        let mut movement = trigger.value.extend(0.0).xzy();
        movement.z = -movement.z;

        // Make speed dependent on camera distance.
        let arm_multiplier = **spring_arm * 0.02;

        **orbit_origin += rotation * movement * arm_multiplier;
    }

    fn zoom(trigger: Trigger<Fired<ZoomCamera>>, mut spring_arm: Single<&mut SpringArm>) {
        // Limit to prevent clipping into the ground.
        ***spring_arm = (***spring_arm - trigger.value).max(0.2);
    }

    fn rotate(
        trigger: Trigger<Fired<RotateCamera>>,
        settings: Res<Settings>,
        mut rotation: Single<&mut OrbitRotation>,
    ) {
        ***rotation += trigger.value;

        let max_y = if settings.developer.free_camera_rotation {
            PI // To avoid flipping when the camera is under ground.
        } else {
            FRAC_PI_2 - 0.01 // To avoid ground intersection.
        };
        let min_y = 0.001; // To avoid flipping when the camera is vertical.
        rotation.y = rotation.y.clamp(min_y, max_y);
    }

    fn apply_transform(camera: Single<(&mut Transform, &OrbitOrigin, &OrbitRotation, &SpringArm)>) {
        let (mut transform, orbit_origin, orbit_rotation, spring_arm) = camera.into_inner();
        transform.translation = orbit_rotation.sphere_pos() * **spring_arm + **orbit_origin;
        transform.look_at(**orbit_origin, Vec3::Y);
    }
}

#[derive(Component)]
#[require(
    OrbitOrigin,
    OrbitRotation,
    SpringArm,
    Name(|| Name::new("Player camera")),
    Camera3d,
    Msaa(|| Msaa::Off),
    Camera(|| Camera { hdr: true, ..Default::default() }),
    TemporalAntiAliasing,
    EnvironmentMapLight,
    ScreenSpaceAmbientOcclusion
)]
pub(super) struct PlayerCamera;

impl InputContext for PlayerCamera {
    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        ctx.bind::<EnableCameraRotation>().to(MouseButton::Middle);
        ctx.bind::<EnablePanCamera>()
            .to((MouseButton::Right, GamepadButton::East));

        ctx.bind::<PanCamera>()
            .to((
                Cardinal {
                    north: &settings.keyboard.camera_forward,
                    east: &settings.keyboard.camera_left,
                    south: &settings.keyboard.camera_backward,
                    west: &settings.keyboard.camera_right,
                },
                (
                    GamepadAxis::LeftStickX,
                    GamepadAxis::LeftStickY.with_modifiers(SwizzleAxis::YXZ),
                )
                    .with_conditions_each(Chord::<EnablePanCamera>::default()),
                Input::mouse_motion()
                    .with_modifiers((
                        Negate::y(),
                        AccumulateBy::<EnablePanCamera>::default(),
                        Scale::splat(0.003),
                    ))
                    .with_conditions(Chord::<EnablePanCamera>::default()),
            ))
            .with_modifiers((
                DeadZone::default(),
                Scale::splat(0.7),
                SmoothNudge::default(),
            ));

        ctx.bind::<RotateCamera>()
            .to((
                Bidirectional {
                    positive: &settings.keyboard.rotate_right,
                    negative: &settings.keyboard.rotate_left,
                },
                GamepadStick::Right,
                Input::mouse_motion()
                    .with_modifiers(Negate::all())
                    .with_modifiers(Scale::splat(0.08))
                    .with_conditions(Chord::<EnableCameraRotation>::default()),
            ))
            .with_modifiers((Scale::splat(0.05), SmoothNudge::default()));

        ctx.bind::<ZoomCamera>()
            .to((
                Bidirectional {
                    positive: &settings.keyboard.zoom_in,
                    negative: &settings.keyboard.zoom_out,
                },
                Bidirectional {
                    positive: GamepadAxis::RightZ,
                    negative: GamepadAxis::LeftZ,
                }
                .with_modifiers_each(Scale::splat(0.1)),
                Input::mouse_wheel().with_modifiers(SwizzleAxis::YXZ),
            ))
            .with_modifiers(SmoothNudge::default());

        ctx
    }
}

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct PanCamera;

#[derive(Debug, InputAction)]
#[input_action(output = f32)]
struct ZoomCamera;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
struct RotateCamera;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct EnableCameraRotation;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
struct EnablePanCamera;

#[derive(Clone, Copy, Debug, EnumIter, IntoPrimitive)]
#[repr(usize)]
enum EnvironmentMap {
    Diffuse,
    Specular,
}

impl AssetCollection for EnvironmentMap {
    type AssetType = Image;

    fn asset_path(&self) -> AssetPath<'static> {
        match self {
            EnvironmentMap::Diffuse => "base/environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2".into(),
            EnvironmentMap::Specular => {
                "base/environment_maps/pisa_specular_rgb9e5_zstd.ktx2".into()
            }
        }
    }
}

/// The origin of a camera.
#[derive(Component, Default, Deref, DerefMut)]
struct OrbitOrigin(Vec3);

/// Camera rotation in `X` and `Z`.
#[derive(Component, Deref, DerefMut)]
struct OrbitRotation(Vec2);

impl OrbitRotation {
    fn sphere_pos(&self) -> Vec3 {
        Quat::from_euler(EulerRot::YXZ, self.x, self.y, 0.0) * Vec3::Y
    }
}

impl Default for OrbitRotation {
    fn default() -> Self {
        Self(Vec2::new(0.0, 60_f32.to_radians()))
    }
}

/// Camera distance.
#[derive(Component, Deref, DerefMut)]
struct SpringArm(f32);

impl Default for SpringArm {
    fn default() -> Self {
        Self(10.0)
    }
}

/// A helper to cast rays from [`PlayerCamera`].
#[derive(SystemParam)]
pub(super) struct CameraCaster<'w, 's> {
    window: Single<'w, &'static Window>,
    cities: Query<'w, 's, &'static GlobalTransform>,
    camera: Option<
        Single<
            'w,
            (&'static Parent, &'static GlobalTransform, &'static Camera),
            With<PlayerCamera>,
        >,
    >,
}

impl CameraCaster<'_, '_> {
    pub(super) fn intersect_ground(&self) -> Option<Vec3> {
        let (parent, &transform, camera) = self.camera.as_deref()?;
        let cursor_pos = self.window.cursor_position()?;
        let ray = camera.viewport_to_world(&transform, cursor_pos).ok()?;
        let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
        let global_point = ray.get_point(distance);
        let city_transform = self.cities.get(***parent).unwrap();
        let local_point = city_transform
            .affine()
            .inverse()
            .transform_point3(global_point);
        Some(local_point)
    }
}
