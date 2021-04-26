use crate::{Vec2, Vec3};

/// Smooths value to a goal using a damped spring.
pub trait SmoothDamp {
    /// Smooths value to a goal using a damped spring.
    ///
    /// `smooth_time` is the expected time to reach the target when at maximum velocity.
    ///
    /// Returns smoothed value and new velocity.
    fn smooth_damp(
        from: Self,
        to: Self,
        velocity: Self,
        smooth_time: f32,
        delta_time: f32,
    ) -> (Self, Self)
    where
        Self: Sized;
}

macro_rules! impl_smooth_damp_scalar {
    ($t:ty, $f:ty) => {
        impl SmoothDamp for $t {
            fn smooth_damp(
                from: $t,
                to: $t,
                velocity: $t,
                smooth_time: f32,
                delta_time: f32,
            ) -> ($t, $t) {
                let smooth_time = <$f>::max(smooth_time as $f, 0.0001); // ensure smooth_time is positive and non-zero
                let delta_time = delta_time as $f;

                // from game programming gems 4, chapter 1.10
                let omega = 2.0 / smooth_time;
                let x = omega * delta_time;

                let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
                // let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x))); // TODO: profile me, both in debug & release

                let change = from - to;
                let temp = (velocity + omega * change) * delta_time;

                (
                    to + (change + temp) * exp,      // position
                    (velocity - omega * temp) * exp, // velocity
                )
            }
        }
    }
}

impl_smooth_damp_scalar! {f32, f32}
impl_smooth_damp_scalar! {f64, f64}
impl_smooth_damp_scalar! {Vec2, f32}
impl_smooth_damp_scalar! {Vec3, f32}

/// Smooths value to a goal using a damped spring limited by a maximum speed.
pub trait SmoothDampMax {
    /// Smooths value to a goal using a damped spring limited by a maximum speed.
    ///
    /// `smooth_time` is the expected time to reach the target when at maximum velocity.
    ///
    /// Returns smoothed value and new velocity.
    fn smooth_damp_max(
        from: Self,
        to: Self,
        velocity: Self,
        max_speed: f32,
        smooth_time: f32,
        delta_time: f32,
    ) -> (Self, Self)
    where
        Self: Sized;
}

macro_rules! impl_smooth_damp_max_scalar {
    ($t:ty) => {
        impl SmoothDampMax for $t {
            fn smooth_damp_max(
                from: $t,
                to: $t,
                velocity: $t,
                max_speed: f32,
                smooth_time: f32,
                delta_time: f32,
            ) -> ($t, $t) {
                let max_speed = <$t>::max(max_speed as $t, 0.0001); // ensure max speed is positive and non-zero
                let smooth_time = <$t>::max(smooth_time as $t, 0.0001); // ensure smooth_time is positive and non-zero
                let delta_time = delta_time as $t;

                // from game programming gems 4, chapter 1.10
                let omega = 2.0 / smooth_time;
                let x = omega * delta_time;

                let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
                // let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x))); // TODO: profile me, both in debug & release

                let max = max_speed * delta_time;
                let change = <$t>::clamp(from - to, -max, max);

                let temp = (velocity + omega * change) * delta_time;

                (
                    to + (change + temp) * exp,      // position
                    (velocity - omega * temp) * exp, // velocity
                )
            }
        }
    }
}

impl_smooth_damp_max_scalar!{f32}
impl_smooth_damp_max_scalar!{f64}

macro_rules! impl_smooth_damp_max_vec {
    ($t:ty) => {
        impl SmoothDampMax for $t {
            fn smooth_damp_max(
                from: $t,
                to: $t,
                velocity: $t,
                max_speed: f32,
                smooth_time: f32,
                delta_time: f32,
            ) -> ($t, $t) {
                let max_speed = f32::max(max_speed, 0.0001); // ensure max speed is positive and non-zero
                let smooth_time = f32::max(smooth_time, 0.0001); // ensure smooth_time is positive and non-zero

                // from game programming gems 4, chapter 1.10
                let omega = 2.0 / smooth_time;
                let x = omega * delta_time;

                let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
                // let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x))); // TODO: profile me, both in debug & release

                let max = max_speed * delta_time;
                let change = (from - to).clamp_length_max(max);

                let temp = (velocity + omega * change) * delta_time;

                (
                    to + (change + temp) * exp,      // position
                    (velocity - omega * temp) * exp, // velocity
                )
            }
        }
    };
}

impl_smooth_damp_max_vec! {Vec2}
impl_smooth_damp_max_vec! {Vec3}
