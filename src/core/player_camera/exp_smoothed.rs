use bevy::prelude::*;

#[derive(Default)]
pub(super) struct ExpSmoothed<T> {
    value: T,
    pub(super) dest: T,
}

impl<T: Copy + Lerp> ExpSmoothed<T> {
    pub(super) fn new(value: T) -> Self {
        Self { value, dest: value }
    }

    pub(super) fn smooth(&mut self, delta_secs: f32) {
        // An ad-hoc multiplier to make default smoothness parameters
        // produce good-looking results. Taken from `dolly` crate.
        const INTERPOLATION_SPEED: f32 = 8.0;

        // Calculate the exponential blending based on frame time.
        let t = 1.0 - (-INTERPOLATION_SPEED * delta_secs).exp();
        self.value = self.value.lerp(self.dest, t);
    }

    pub(super) fn value(&self) -> T {
        self.value
    }
}

pub(super) trait Lerp {
    fn lerp(self, other: Self, t: f32) -> Self;
}

impl Lerp for Vec3 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Lerp for Vec2 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self.lerp(other, t)
    }
}

impl Lerp for f32 {
    fn lerp(self, other: Self, t: f32) -> Self {
        self + t * (other - self)
    }
}
