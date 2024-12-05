use std::f32::consts::{FRAC_PI_2, PI};

use bevy::{
    asset::AssetPath,
    core_pipeline::{
        bloom::BloomSettings, experimental::taa::TemporalAntiAliasBundle, prepass::NormalPrepass,
        tonemapping::Tonemapping,
    },
    ecs::system::SystemParam,
    pbr::ScreenSpaceAmbientOcclusionSettings,
    prelude::*,
};
use bevy_enhanced_input::prelude::*;
use num_enum::IntoPrimitive;
use strum::EnumIter;

use crate::{
    asset::collection::{AssetCollection, Collection},
    common_conditions::{in_any_state, observer_in_state},
    game_world::WorldState,
    settings::Settings,
};

pub(super) struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Collection<EnvironmentMap>>()
            .add_input_context::<PlayerCamera>()
            .observe(Self::pan)
            .observe(Self::zoom)
            .observe(Self::rotate)
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
    fn pan(
        trigger: Trigger<Fired<PanCamera>>,
        world_state: Option<Res<State<WorldState>>>,
        mut cameras: Query<(&mut OrbitOrigin, &Transform, &SpringArm), With<PlayerCamera>>,
    ) {
        if observer_in_state(world_state, WorldState::FamilyEditor) {
            return;
        }

        // Calculate direction without camera's tilt.
        let (mut orbit_origin, transform, spring_arm) = cameras.single_mut();
        let forward = transform.forward();
        let camera_dir = Vec3::new(forward.x, 0.0, forward.z).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Z, camera_dir);

        // Movement consists of X and -Z components, so swap Y and Z with negation.
        let event = trigger.event();
        let mut movement = event.value.extend(0.0).xzy();
        movement.z = -movement.z;

        // Make speed dependent on camera distance.
        let arm_multiplier = **spring_arm * 0.02;

        **orbit_origin += rotation * movement * arm_multiplier;
    }

    fn zoom(
        trigger: Trigger<Fired<ZoomCamera>>,
        mut cameras: Query<&mut SpringArm, With<PlayerCamera>>,
    ) {
        let event = trigger.event();
        let mut spring_arm = cameras.single_mut();
        // Limit to prevent clipping into the ground.
        **spring_arm = (**spring_arm - event.value).max(0.2);
    }

    fn rotate(
        trigger: Trigger<Fired<RotateCamera>>,
        mut cameras: Query<&mut OrbitRotation, With<PlayerCamera>>,
        settings: Res<Settings>,
    ) {
        let event = trigger.event();
        let mut rotation = cameras.single_mut();
        **rotation += event.value;

        let max_y = if settings.developer.free_camera_rotation {
            PI // To avoid flipping when the camera is under ground.
        } else {
            FRAC_PI_2 - 0.01 // To avoid ground intersection.
        };
        let min_y = 0.001; // To avoid flipping when the camera is vertical.
        rotation.y = rotation.y.clamp(min_y, max_y);
    }

    fn apply_transform(
        mut cameras: Query<
            (&mut Transform, &OrbitOrigin, &OrbitRotation, &SpringArm),
            With<PlayerCamera>,
        >,
    ) {
        let (mut transform, orbit_origin, orbit_rotation, spring_arm) = cameras.single_mut();
        transform.translation = orbit_rotation.sphere_pos() * **spring_arm + **orbit_origin;
        transform.look_at(**orbit_origin, Vec3::Y);
    }
}

#[derive(Bundle)]
pub(crate) struct PlayerCameraBundle {
    orbit_origin: OrbitOrigin,
    orbit_rotation: OrbitRotation,
    spring_arm: SpringArm,
    player_camera: PlayerCamera,
    camera_3d_bundle: Camera3dBundle,
    taa_bundle: TemporalAntiAliasBundle,
    bloom: BloomSettings,
    environment_map: EnvironmentMapLight,

    /// Needed for SSAO.
    ///
    /// The bundle can't be included because TAA and SSAO bundles both contain [`DepthPrepass`].
    normal_prepass: NormalPrepass,
    ssao_settings: ScreenSpaceAmbientOcclusionSettings,
}

impl PlayerCameraBundle {
    pub(crate) fn new(environment_map: &Collection<EnvironmentMap>) -> Self {
        Self {
            orbit_origin: Default::default(),
            orbit_rotation: Default::default(),
            spring_arm: Default::default(),
            player_camera: PlayerCamera,
            camera_3d_bundle: Camera3dBundle {
                tonemapping: Tonemapping::AcesFitted,
                camera: Camera {
                    hdr: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            taa_bundle: Default::default(),
            ssao_settings: Default::default(),
            normal_prepass: Default::default(),
            bloom: BloomSettings::default(),
            environment_map: EnvironmentMapLight {
                diffuse_map: environment_map.handle(EnvironmentMap::Diffuse),
                specular_map: environment_map.handle(EnvironmentMap::Specular),
                intensity: 1750.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, EnumIter, IntoPrimitive)]
#[repr(usize)]
pub(super) enum EnvironmentMap {
    Diffuse,
    Specular,
}

impl AssetCollection for EnvironmentMap {
    type AssetType = Image;

    fn asset_path(&self) -> AssetPath<'static> {
        match self {
            EnvironmentMap::Diffuse => "environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2".into(),
            EnvironmentMap::Specular => "environment_maps/pisa_specular_rgb9e5_zstd.ktx2".into(),
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

#[derive(Component)]
pub(super) struct PlayerCamera;

impl InputContext for PlayerCamera {
    fn context_instance(world: &World, _entity: Entity) -> ContextInstance {
        let mut ctx = ContextInstance::default();
        let settings = world.resource::<Settings>();

        ctx.bind::<EnableCameraRotation>().to(MouseButton::Middle);
        ctx.bind::<EnablePanCamera>()
            .to((MouseButton::Right, GamepadButtonType::East));

        ctx.bind::<PanCamera>()
            .to((
                Cardinal {
                    north: &settings.keyboard.camera_forward,
                    east: &settings.keyboard.camera_left,
                    south: &settings.keyboard.camera_backward,
                    west: &settings.keyboard.camera_right,
                },
                // TODO 0.15: replace with condition on set.
                (
                    GamepadAxisType::LeftStickX
                        .with_conditions(Chord::<EnablePanCamera>::default()),
                    GamepadAxisType::LeftStickY
                        .with_modifiers(SwizzleAxis::YXZ)
                        .with_conditions(Chord::<EnablePanCamera>::default()),
                ),
                Input::mouse_motion()
                    .with_modifiers((
                        Negate::y(true),
                        AccumulateBy::<EnablePanCamera>::default(),
                        Scale::splat(0.003),
                    ))
                    .with_conditions(Chord::<EnablePanCamera>::default()),
            ))
            .with_modifiers((DeadZone::default(), Scale::splat(0.7), DeltaLerp::default()));

        ctx.bind::<RotateCamera>()
            .to((
                Biderectional {
                    positive: &settings.keyboard.rotate_right,
                    negative: &settings.keyboard.rotate_left,
                },
                GamepadStick::Right,
                Input::mouse_motion()
                    .with_modifiers(Scale::splat(0.05))
                    .with_conditions(Chord::<EnableCameraRotation>::default()),
            ))
            .with_modifiers((Scale::splat(0.05), DeltaLerp::default()));

        ctx.bind::<ZoomCamera>()
            .to((
                Biderectional {
                    positive: &settings.keyboard.zoom_in,
                    negative: &settings.keyboard.zoom_out,
                },
                // TODO 0.15: scale set.
                Biderectional {
                    positive: GamepadAxisType::RightZ.with_modifiers(Scale::splat(0.1)),
                    negative: GamepadAxisType::LeftZ.with_modifiers(Scale::splat(0.1)),
                },
                Input::mouse_wheel().with_modifiers(SwizzleAxis::YXZ),
            ))
            .with_modifiers(DeltaLerp::default());

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

/// A helper to cast rays from [`PlayerCamera`].
#[derive(SystemParam)]
pub(super) struct CameraCaster<'w, 's> {
    windows: Query<'w, 's, &'static Window>,
    cities: Query<'w, 's, &'static GlobalTransform>,
    cameras: Query<
        'w,
        's,
        (&'static Parent, &'static GlobalTransform, &'static Camera),
        With<PlayerCamera>,
    >,
}

impl CameraCaster<'_, '_> {
    pub(super) fn ray(&self) -> Option<Ray3d> {
        let (_, &transform, camera) = self.cameras.get_single().ok()?;
        self.ray_from(transform, camera)
    }

    pub(super) fn intersect_ground(&self) -> Option<Vec3> {
        let (parent, &transform, camera) = self.cameras.get_single().ok()?;
        let ray = self.ray_from(transform, camera)?;
        let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y))?;
        let global_point = ray.get_point(distance);
        let city_transform = self.cities.get(**parent).unwrap();
        let local_point = city_transform
            .affine()
            .inverse()
            .transform_point3(global_point);
        Some(local_point)
    }

    fn ray_from(&self, transform: GlobalTransform, camera: &Camera) -> Option<Ray3d> {
        let cursor_pos = self.windows.single().cursor_position()?;
        camera.viewport_to_world(&transform, cursor_pos)
    }
}
