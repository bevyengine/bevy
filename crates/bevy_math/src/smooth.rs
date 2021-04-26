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

impl SmoothDamp for f32 {
    fn smooth_damp(
        from: f32,
        to: f32,
        velocity: f32,
        smooth_time: f32,
        delta_time: f32,
    ) -> (f32, f32) {
        let smooth_time = f32::max(smooth_time, 0.0001); // ensure smooth_time is positive and non-zero

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

impl SmoothDamp for f64 {
    fn smooth_damp(
        from: f64,
        to: f64,
        velocity: f64,
        smooth_time: f32,
        delta_time: f32,
    ) -> (f64, f64) {
        let smooth_time = f64::max(smooth_time as f64, 0.0001); // ensure smooth_time is positive and non-zero
        let delta_time = delta_time as f64;

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

impl SmoothDamp for Vec2 {
    fn smooth_damp(
        from: Vec2,
        to: Vec2,
        velocity: Vec2,
        smooth_time: f32,
        delta_time: f32,
    ) -> (Vec2, Vec2) {
        let (x, vx) = f32::smooth_damp(from.x, to.x, velocity.x, smooth_time, delta_time);
        let (y, vy) = f32::smooth_damp(from.y, to.y, velocity.y, smooth_time, delta_time);
        (Vec2::new(x, y), Vec2::new(vx, vy))
    }
}

impl SmoothDamp for Vec3 {
    fn smooth_damp(
        from: Vec3,
        to: Vec3,
        velocity: Vec3,
        smooth_time: f32,
        delta_time: f32,
    ) -> (Vec3, Vec3) {
        let (x, vx) = f32::smooth_damp(from.x, to.x, velocity.x, smooth_time, delta_time);
        let (y, vy) = f32::smooth_damp(from.y, to.y, velocity.y, smooth_time, delta_time);
        let (z, vz) = f32::smooth_damp(from.z, to.z, velocity.z, smooth_time, delta_time);
        (Vec3::new(x, y, z), Vec3::new(vx, vy, vz))
    }
}

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

impl SmoothDampMax for f32 {
    fn smooth_damp_max(
        from: f32,
        to: f32,
        velocity: f32,
        max_speed: f32,
        smooth_time: f32,
        delta_time: f32,
    ) -> (f32, f32) {
        let max_speed = f32::max(max_speed, 0.0001); // ensure max speed is positive and non-zero
        let smooth_time = f32::max(smooth_time, 0.0001); // ensure smooth_time is positive and non-zero

        // from game programming gems 4, chapter 1.10
        let omega = 2.0 / smooth_time;
        let x = omega * delta_time;

        let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
        // let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x))); // TODO: profile me, both in debug & release

        let max = max_speed * delta_time;
        let change = f32::clamp(from - to, -max, max);

        let temp = (velocity + omega * change) * delta_time;

        (
            to + (change + temp) * exp,      // position
            (velocity - omega * temp) * exp, // velocity
        )
    }
}

impl SmoothDampMax for f64 {
    fn smooth_damp_max(
        from: f64,
        to: f64,
        velocity: f64,
        max_speed: f32,
        smooth_time: f32,
        delta_time: f32,
    ) -> (f64, f64) {
        let max_speed = f32::max(max_speed, 0.0001); // ensure max speed is positive and non-zero
        let smooth_time = f64::max(smooth_time as f64, 0.0001); // ensure smooth_time is positive and non-zero
        let delta_time = delta_time as f64;

        // from game programming gems 4, chapter 1.10
        let omega = 2.0 / smooth_time;
        let x = omega * delta_time;

        let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
        // let exp = 1.0 / (1.0 + x * (1.0 + x * (0.48 + 0.235 * x))); // TODO: profile me, both in debug & release

        let max = (max_speed as f64) * delta_time;
        let change = f64::clamp(from - to, -max, max);

        let temp = (velocity + omega * change) * delta_time;

        (
            to + (change + temp) * exp,      // position
            (velocity - omega * temp) * exp, // velocity
        )
    }
}

impl SmoothDampMax for Vec2 {
    fn smooth_damp_max(
        from: Vec2,
        to: Vec2,
        velocity: Vec2,
        max_speed: f32,
        smooth_time: f32,
        delta_time: f32,
    ) -> (Vec2, Vec2) {
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

impl SmoothDampMax for Vec3 {
    fn smooth_damp_max(
        from: Vec3,
        to: Vec3,
        velocity: Vec3,
        max_speed: f32,
        smooth_time: f32,
        delta_time: f32,
    ) -> (Vec3, Vec3) {
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
