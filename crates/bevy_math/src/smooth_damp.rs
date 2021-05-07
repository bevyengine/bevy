use crate::{Vec2, Vec3};

/// Smooths value to a goal using a damped spring.
pub trait SmoothDamp {
    /// Smooths value to a goal using a damped spring.
    ///
    /// `smooth_time` is the expected time to reach the target when at maximum velocity.
    ///
    /// Returns smoothed value and new velocity.
    ///
    /// # Panics
    /// Panics if `smooth_time <= 0.0`.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::prelude::{Vec3, Quat};
    /// # use bevy_math::SmoothDamp;
    /// # struct Transform {
    /// #     translation: Vec3,
    /// #     rotation: Quat,
    /// #     scale: Vec3
    /// # }
    /// struct SmoothTransform {
    ///     pub smoothness: f32,
    ///     pub target: Vec3,   
    ///     velocity: Vec3   
    /// }
    ///
    /// fn smooth_transform_update(dt: f32, transform: &mut Transform, smoother: &mut SmoothTransform) {
    ///     let (p, v) = Vec3::smooth_damp(
    ///         transform.translation,
    ///         smoother.target,
    ///         smoother.velocity,
    ///         smoother.smoothness,
    ///         dt,
    ///     );
    ///     transform.translation = p;
    ///     smoother.velocity = v;
    ///     // When destructured assignement will be supported by Rust:
    ///     // (transform.translation, smoother.velocity) =
    ///     //     Vec3::smooth_damp(
    ///     //         transform.translation,
    ///     //         smoother.target,
    ///     //         smoother.velocity,
    ///     //         smoother.smoothness,
    ///     //         dt,
    ///     //      );
    /// }
    /// ```
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

macro_rules! impl_smooth_damp {
    ($t:ty, $f:ty) => {
        impl SmoothDamp for $t {
            fn smooth_damp(
                from: $t,
                to: $t,
                velocity: $t,
                smooth_time: f32,
                delta_time: f32,
            ) -> ($t, $t) {
                assert!(smooth_time > 0.0);
                let smooth_time = smooth_time as $f;

                let delta_time = delta_time as $f;

                // from game programming gems 4, chapter 1.10
                let omega = 2.0 / smooth_time;
                let x = omega * delta_time;

                // fast and good enough approximation of exp(x)
                let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x)));

                let change = from - to;
                let temp = (velocity + omega * change) * delta_time;

                (
                    to + (change + temp) * exp,      // position
                    (velocity - omega * temp) * exp, // velocity
                )
            }
        }
    };
}

impl_smooth_damp! {f32, f32}
impl_smooth_damp! {f64, f64}
impl_smooth_damp! {Vec2, f32}
impl_smooth_damp! {Vec3, f32}

/// Smooths value to a goal using a damped spring limited by a maximum speed.
pub trait SmoothDampMax {
    /// Smooths value to a goal using a damped spring limited by a maximum speed.
    ///
    /// `smooth_time` is the expected time to reach the target when at maximum velocity.
    ///
    /// Returns smoothed value and new velocity.
    ///
    /// # Panics
    /// Panics if `smooth_time <= 0.0` or `max_speed <= 0.0`.
    ///
    /// # Example
    /// ```
    /// # use bevy_math::prelude::{Vec3, Quat};
    /// # use bevy_math::SmoothDampMax;
    /// # struct Transform {
    /// #     translation: Vec3,
    /// #     rotation: Quat,
    /// #     scale: Vec3
    /// # }
    /// struct SmoothTransform {
    ///     pub smoothness: f32,
    ///     pub max_speed: f32,
    ///     pub target: Vec3,   
    ///     velocity: Vec3   
    /// }
    ///
    /// fn smooth_transform_update(dt: f32, transform: &mut Transform, smoother: &mut SmoothTransform) {
    ///     let (p, v) = Vec3::smooth_damp_max(
    ///         transform.translation,
    ///         smoother.target,
    ///         smoother.velocity,
    ///         smoother.max_speed,
    ///         smoother.smoothness,
    ///         dt,
    ///     );
    ///     transform.translation = p;
    ///     smoother.velocity = v;
    ///     // When destructured assignement will be supported by Rust:
    ///     // (transform.translation, smoother.velocity) =
    ///     //     Vec3::smooth_damp_max(
    ///     //         transform.translation,
    ///     //         smoother.target,
    ///     //         smoother.velocity,
    ///     //         smoother.max_speed,
    ///     //         smoother.smoothness,
    ///     //         dt,
    ///     //      );
    /// }
    /// ```
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

macro_rules! impl_smooth_damp_max {
    ($t:ty, $f:ty, $clamp:expr) => {
        impl SmoothDampMax for $t {
            fn smooth_damp_max(
                from: $t,
                to: $t,
                velocity: $t,
                max_speed: f32,
                smooth_time: f32,
                delta_time: f32,
            ) -> ($t, $t) {
                assert!(max_speed > 0.0);
                let max_speed = max_speed as $f;

                assert!(smooth_time > 0.0);
                let smooth_time = smooth_time as $f;

                let delta_time = delta_time as $f;

                // from game programming gems 4, chapter 1.10
                let omega = 2.0 / smooth_time;
                let x = omega * delta_time;

                // fast and good enough approximation of exp(x)
                let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x)));

                let max = max_speed * smooth_time;
                let change = from - to;
                let change = $clamp(change, max);
                let to = from - change;

                let temp = (velocity + omega * change) * delta_time;

                (
                    to + (change + temp) * exp,      // position
                    (velocity - omega * temp) * exp, // velocity
                )
            }
        }
    };
}

impl_smooth_damp_max! {f32, f32, |change, max:f32| { f32::clamp(change, -max, max) }}
impl_smooth_damp_max! {f64, f64, |change, max:f64| { f64::clamp(change, -max, max) }}
impl_smooth_damp_max! {Vec2, f32, |change:Vec2, max| { change.clamp_length_max(max) }}
impl_smooth_damp_max! {Vec3, f32, |change:Vec3, max| { change.clamp_length_max(max) }}
