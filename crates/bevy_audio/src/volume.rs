use bevy_ecs::prelude::*;
use bevy_math::ops;
use bevy_reflect::prelude::*;

/// Use this [`Resource`] to control the global volume of all audio.
///
/// Note: Changing [`GlobalVolume`] does not affect already playing audio.
#[derive(Resource, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Resource, Debug, Default, Clone)]
pub struct GlobalVolume {
    /// The global volume of all audio.
    pub volume: Volume,
}

impl From<Volume> for GlobalVolume {
    fn from(volume: Volume) -> Self {
        Self { volume }
    }
}

impl GlobalVolume {
    /// Create a new [`GlobalVolume`] with the given volume.
    pub fn new(volume: Volume) -> Self {
        Self { volume }
    }
}

/// A [`Volume`] represents an audio source's volume level.
///
/// To create a new [`Volume`] from a linear scale value, use
/// [`Volume::Linear`].
///
/// To create a new [`Volume`] from decibels, use [`Volume::Decibels`].
#[derive(Clone, Copy, Debug, Reflect)]
#[reflect(Clone, Debug, PartialEq)]
pub enum Volume {
    /// Create a new [`Volume`] from the given volume in the linear scale.
    ///
    /// In a linear scale, the value `1.0` represents the "normal" volume,
    /// meaning the audio is played at its original level. Values greater than
    /// `1.0` increase the volume, while values between `0.0` and `1.0` decrease
    /// the volume. A value of `0.0` effectively mutes the audio.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_audio::Volume;
    /// # use bevy_math::ops;
    /// #
    /// # const EPSILON: f32 = 0.01;
    ///
    /// let volume = Volume::Linear(0.5);
    /// assert_eq!(volume.to_linear(), 0.5);
    /// assert!(ops::abs(volume.to_decibels() - -6.0206) < EPSILON);
    ///
    /// let volume = Volume::Linear(0.0);
    /// assert_eq!(volume.to_linear(), 0.0);
    /// assert_eq!(volume.to_decibels(), f32::NEG_INFINITY);
    ///
    /// let volume = Volume::Linear(1.0);
    /// assert_eq!(volume.to_linear(), 1.0);
    /// assert!(ops::abs(volume.to_decibels() - 0.0) < EPSILON);
    /// ```
    Linear(f32),
    /// Create a new [`Volume`] from the given volume in decibels.
    ///
    /// In a decibel scale, the value `0.0` represents the "normal" volume,
    /// meaning the audio is played at its original level. Values greater than
    /// `0.0` increase the volume, while values less than `0.0` decrease the
    /// volume. A value of [`f32::NEG_INFINITY`] decibels effectively mutes the
    /// audio.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_audio::Volume;
    /// # use bevy_math::ops;
    /// #
    /// # const EPSILON: f32 = 0.01;
    ///
    /// let volume = Volume::Decibels(-5.998);
    /// assert!(ops::abs(volume.to_linear() - 0.5) < EPSILON);
    ///
    /// let volume = Volume::Decibels(f32::NEG_INFINITY);
    /// assert_eq!(volume.to_linear(), 0.0);
    ///
    /// let volume = Volume::Decibels(0.0);
    /// assert_eq!(volume.to_linear(), 1.0);
    ///
    /// let volume = Volume::Decibels(20.0);
    /// assert_eq!(volume.to_linear(), 10.0);
    /// ```
    Decibels(f32),
}

impl Default for Volume {
    fn default() -> Self {
        Self::Linear(1.0)
    }
}

impl PartialEq for Volume {
    fn eq(&self, other: &Self) -> bool {
        use Volume::{Decibels, Linear};

        match (self, other) {
            (Linear(a), Linear(b)) => a.abs() == b.abs(),
            (Decibels(a), Decibels(b)) => a == b,
            (a, b) => a.to_decibels() == b.to_decibels(),
        }
    }
}

impl PartialOrd for Volume {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        use Volume::{Decibels, Linear};

        Some(match (self, other) {
            (Linear(a), Linear(b)) => a.abs().total_cmp(&b.abs()),
            (Decibels(a), Decibels(b)) => a.total_cmp(b),
            (a, b) => a.to_decibels().total_cmp(&b.to_decibels()),
        })
    }
}

#[inline]
fn decibels_to_linear(decibels: f32) -> f32 {
    ops::powf(10.0f32, decibels / 20.0)
}

#[inline]
fn linear_to_decibels(linear: f32) -> f32 {
    20.0 * ops::log10(linear.abs())
}

impl Volume {
    /// Returns the volume in linear scale as a float.
    pub fn to_linear(&self) -> f32 {
        match self {
            Self::Linear(v) => v.abs(),
            Self::Decibels(v) => decibels_to_linear(*v),
        }
    }

    /// Returns the volume in decibels as a float.
    ///
    /// If the volume is silent / off / muted, i.e., its underlying linear scale
    /// is `0.0`, this method returns negative infinity.
    pub fn to_decibels(&self) -> f32 {
        match self {
            Self::Linear(v) => linear_to_decibels(*v),
            Self::Decibels(v) => *v,
        }
    }

    /// The silent volume. Also known as "off" or "muted".
    pub const SILENT: Self = Volume::Linear(0.0);

    /// Increases the volume by the specified percentage.
    ///
    /// This method works in the linear domain, where a 100% increase
    /// means doubling the volume (equivalent to +6.02dB).
    ///
    /// # Arguments
    /// * `percentage` - The percentage to increase (50.0 means 50% increase)
    ///
    /// # Examples
    /// ```
    /// use bevy_audio::Volume;
    ///
    /// let volume = Volume::Linear(1.0);
    /// let increased = volume.increase_by_percentage(100.0);
    /// assert_eq!(increased.to_linear(), 2.0);
    /// ```
    pub fn increase_by_percentage(&self, percentage: f32) -> Self {
        let factor = 1.0 + (percentage / 100.0);
        Volume::Linear(self.to_linear() * factor)
    }

    /// Decreases the volume by the specified percentage.
    ///
    /// This method works in the linear domain, where a 50% decrease
    /// means halving the volume (equivalent to -6.02dB).
    ///
    /// # Arguments
    /// * `percentage` - The percentage to decrease (50.0 means 50% decrease)
    ///
    /// # Examples
    /// ```
    /// use bevy_audio::Volume;
    ///
    /// let volume = Volume::Linear(1.0);
    /// let decreased = volume.decrease_by_percentage(50.0);
    /// assert_eq!(decreased.to_linear(), 0.5);
    /// ```
    pub fn decrease_by_percentage(&self, percentage: f32) -> Self {
        let factor = 1.0 - (percentage / 100.0).clamp(0.0, 1.0);
        Volume::Linear(self.to_linear() * factor)
    }

    /// Scales the volume to a specific linear factor relative to the current volume.
    ///
    /// This is different from `adjust_by_linear` as it sets the volume to be
    /// exactly the factor times the original volume, rather than applying
    /// the factor to the current volume.
    ///
    /// # Arguments
    /// * `factor` - The scaling factor (2.0 = twice as loud, 0.5 = half as loud)
    ///
    /// # Examples
    /// ```
    /// use bevy_audio::Volume;
    ///
    /// let volume = Volume::Linear(0.8);
    /// let scaled = volume.scale_to_factor(1.25);
    /// assert_eq!(scaled.to_linear(), 1.0);
    /// ```
    pub fn scale_to_factor(&self, factor: f32) -> Self {
        Volume::Linear(self.to_linear() * factor)
    }

    /// Creates a fade effect by interpolating between current volume and target volume.
    ///
    /// This method performs linear interpolation in the linear domain, which
    /// provides a more natural-sounding fade effect.
    ///
    /// # Arguments
    /// * `target` - The target volume to fade towards
    /// * `factor` - The interpolation factor (0.0 = current volume, 1.0 = target volume)
    ///
    /// # Examples
    /// ```
    /// use bevy_audio::Volume;
    ///
    /// let current = Volume::Linear(1.0);
    /// let target = Volume::Linear(0.0);
    /// let faded = current.fade_towards(target, 0.5);
    /// assert_eq!(faded.to_linear(), 0.5);
    /// ```
    pub fn fade_towards(&self, target: Volume, factor: f32) -> Self {
        let current_linear = self.to_linear();
        let target_linear = target.to_linear();
        let factor_clamped = factor.clamp(0.0, 1.0);

        let interpolated = current_linear + (target_linear - current_linear) * factor_clamped;
        Volume::Linear(interpolated)
    }
}

impl core::ops::Mul<Self> for Volume {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        use Volume::{Decibels, Linear};

        match (self, rhs) {
            (Linear(a), Linear(b)) => Linear(a * b),
            (Decibels(a), Decibels(b)) => Decibels(a + b),
            // {Linear, Decibels} favors the left hand side of the operation by
            // first converting the right hand side to the same type as the left
            // hand side and then performing the operation.
            (Linear(..), Decibels(db)) => self * Linear(decibels_to_linear(db)),
            (Decibels(..), Linear(l)) => self * Decibels(linear_to_decibels(l)),
        }
    }
}

impl core::ops::MulAssign<Self> for Volume {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl core::ops::Div<Self> for Volume {
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        use Volume::{Decibels, Linear};

        match (self, rhs) {
            (Linear(a), Linear(b)) => Linear(a / b),
            (Decibels(a), Decibels(b)) => Decibels(a - b),
            // {Linear, Decibels} favors the left hand side of the operation by
            // first converting the right hand side to the same type as the left
            // hand side and then performing the operation.
            (Linear(..), Decibels(db)) => self / Linear(decibels_to_linear(db)),
            (Decibels(..), Linear(l)) => self / Decibels(linear_to_decibels(l)),
        }
    }
}

impl core::ops::DivAssign<Self> for Volume {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::Volume::{self, Decibels, Linear};

    /// Based on [Wikipedia's Decibel article].
    ///
    /// [Wikipedia's Decibel article]: https://web.archive.org/web/20230810185300/https://en.wikipedia.org/wiki/Decibel
    const DECIBELS_LINEAR_TABLE: [(f32, f32); 27] = [
        (100., 100000.),
        (90., 31623.),
        (80., 10000.),
        (70., 3162.),
        (60., 1000.),
        (50., 316.2),
        (40., 100.),
        (30., 31.62),
        (20., 10.),
        (10., 3.162),
        (5.998, 1.995),
        (3.003, 1.413),
        (1.002, 1.122),
        (0., 1.),
        (-1.002, 0.891),
        (-3.003, 0.708),
        (-5.998, 0.501),
        (-10., 0.3162),
        (-20., 0.1),
        (-30., 0.03162),
        (-40., 0.01),
        (-50., 0.003162),
        (-60., 0.001),
        (-70., 0.0003162),
        (-80., 0.0001),
        (-90., 0.00003162),
        (-100., 0.00001),
    ];

    #[test]
    fn volume_conversion() {
        for (db, linear) in DECIBELS_LINEAR_TABLE {
            for volume in [Linear(linear), Decibels(db), Linear(-linear)] {
                let db_test = volume.to_decibels();
                let linear_test = volume.to_linear();

                let db_delta = db_test - db;
                let linear_relative_delta = (linear_test - linear) / linear;

                assert!(
                    db_delta.abs() < 1e-2,
                    "Expected ~{db}dB, got {db_test}dB (delta {db_delta})",
                );
                assert!(
                    linear_relative_delta.abs() < 1e-3,
                    "Expected ~{linear}, got {linear_test} (relative delta {linear_relative_delta})",
                );
            }
        }
    }

    #[test]
    fn volume_conversion_special() {
        assert!(
            Decibels(f32::INFINITY).to_linear().is_infinite(),
            "Infinite decibels is equivalent to infinite linear scale"
        );
        assert!(
            Linear(f32::INFINITY).to_decibels().is_infinite(),
            "Infinite linear scale is equivalent to infinite decibels"
        );

        assert!(
            Linear(f32::NEG_INFINITY).to_decibels().is_infinite(),
            "Negative infinite linear scale is equivalent to infinite decibels"
        );
        assert_eq!(
            Decibels(f32::NEG_INFINITY).to_linear().abs(),
            0.0,
            "Negative infinity decibels is equivalent to zero linear scale"
        );

        assert!(
            Linear(0.0).to_decibels().is_infinite(),
            "Zero linear scale is equivalent to negative infinity decibels"
        );
        assert!(
            Linear(-0.0).to_decibels().is_infinite(),
            "Negative zero linear scale is equivalent to negative infinity decibels"
        );

        assert!(
            Decibels(f32::NAN).to_linear().is_nan(),
            "NaN decibels is equivalent to NaN linear scale"
        );
        assert!(
            Linear(f32::NAN).to_decibels().is_nan(),
            "NaN linear scale is equivalent to NaN decibels"
        );
    }

    #[test]
    fn test_increase_by_percentage() {
        let volume = Linear(1.0);

        // 100% increase should double the volume
        let increased = volume.increase_by_percentage(100.0);
        assert_eq!(increased.to_linear(), 2.0);

        // 50% increase
        let increased = volume.increase_by_percentage(50.0);
        assert_eq!(increased.to_linear(), 1.5);
    }

    #[test]
    fn test_decrease_by_percentage() {
        let volume = Linear(1.0);

        // 50% decrease should halve the volume
        let decreased = volume.decrease_by_percentage(50.0);
        assert_eq!(decreased.to_linear(), 0.5);

        // 25% decrease
        let decreased = volume.decrease_by_percentage(25.0);
        assert_eq!(decreased.to_linear(), 0.75);

        // 100% decrease should result in silence
        let decreased = volume.decrease_by_percentage(100.0);
        assert_eq!(decreased.to_linear(), 0.0);
    }

    #[test]
    fn test_scale_to_factor() {
        let volume = Linear(0.8);
        let scaled = volume.scale_to_factor(1.25);
        assert_eq!(scaled.to_linear(), 1.0);
    }

    #[test]
    fn test_fade_towards() {
        let current = Linear(1.0);
        let target = Linear(0.0);

        // 50% fade should result in 0.5 linear volume
        let faded = current.fade_towards(target, 0.5);
        assert_eq!(faded.to_linear(), 0.5);

        // 0% fade should keep current volume
        let faded = current.fade_towards(target, 0.0);
        assert_eq!(faded.to_linear(), 1.0);

        // 100% fade should reach target volume
        let faded = current.fade_towards(target, 1.0);
        assert_eq!(faded.to_linear(), 0.0);
    }

    #[test]
    fn test_decibel_math_properties() {
        let volume = Linear(1.0);

        // Adding 20dB should multiply linear volume by 10
        let adjusted = volume * Decibels(20.0);
        assert_approx_eq(adjusted, Linear(10.0));

        // Subtracting 20dB should divide linear volume by 10
        let adjusted = volume / Decibels(20.0);
        assert_approx_eq(adjusted, Linear(0.1));
    }

    fn assert_approx_eq(a: Volume, b: Volume) {
        const EPSILON: f32 = 0.0001;

        match (a, b) {
            (Decibels(a), Decibels(b)) | (Linear(a), Linear(b)) => assert!(
                (a - b).abs() < EPSILON,
                "Expected {a:?} to be approximately equal to {b:?}",
            ),
            (a, b) => assert!(
                (a.to_decibels() - b.to_decibels()).abs() < EPSILON,
                "Expected {a:?} to be approximately equal to {b:?}",
            ),
        }
    }

    #[test]
    fn volume_ops_mul() {
        // Linear to Linear.
        assert_approx_eq(Linear(0.5) * Linear(0.5), Linear(0.25));
        assert_approx_eq(Linear(0.5) * Linear(0.1), Linear(0.05));
        assert_approx_eq(Linear(0.5) * Linear(-0.5), Linear(-0.25));

        // Decibels to Decibels.
        assert_approx_eq(Decibels(0.0) * Decibels(0.0), Decibels(0.0));
        assert_approx_eq(Decibels(6.0) * Decibels(6.0), Decibels(12.0));
        assert_approx_eq(Decibels(-6.0) * Decibels(-6.0), Decibels(-12.0));

        // {Linear, Decibels} favors the left hand side of the operation.
        assert_approx_eq(Linear(0.5) * Decibels(0.0), Linear(0.5));
        assert_approx_eq(Decibels(0.0) * Linear(0.501), Decibels(-6.003246));
    }

    #[test]
    fn volume_ops_mul_assign() {
        // Linear to Linear.
        let mut volume = Linear(0.5);
        volume *= Linear(0.5);
        assert_approx_eq(volume, Linear(0.25));

        // Decibels to Decibels.
        let mut volume = Decibels(6.0);
        volume *= Decibels(6.0);
        assert_approx_eq(volume, Decibels(12.0));

        // {Linear, Decibels} favors the left hand side of the operation.
        let mut volume = Linear(0.5);
        volume *= Decibels(0.0);
        assert_approx_eq(volume, Linear(0.5));
        let mut volume = Decibels(0.0);
        volume *= Linear(0.501);
        assert_approx_eq(volume, Decibels(-6.003246));
    }

    #[test]
    fn volume_ops_div() {
        // Linear to Linear.
        assert_approx_eq(Linear(0.5) / Linear(0.5), Linear(1.0));
        assert_approx_eq(Linear(0.5) / Linear(0.1), Linear(5.0));
        assert_approx_eq(Linear(0.5) / Linear(-0.5), Linear(-1.0));

        // Decibels to Decibels.
        assert_approx_eq(Decibels(0.0) / Decibels(0.0), Decibels(0.0));
        assert_approx_eq(Decibels(6.0) / Decibels(6.0), Decibels(0.0));
        assert_approx_eq(Decibels(-6.0) / Decibels(-6.0), Decibels(0.0));

        // {Linear, Decibels} favors the left hand side of the operation.
        assert_approx_eq(Linear(0.5) / Decibels(0.0), Linear(0.5));
        assert_approx_eq(Decibels(0.0) / Linear(0.501), Decibels(6.003246));
    }

    #[test]
    fn volume_ops_div_assign() {
        // Linear to Linear.
        let mut volume = Linear(0.5);
        volume /= Linear(0.5);
        assert_approx_eq(volume, Linear(1.0));

        // Decibels to Decibels.
        let mut volume = Decibels(6.0);
        volume /= Decibels(6.0);
        assert_approx_eq(volume, Decibels(0.0));

        // {Linear, Decibels} favors the left hand side of the operation.
        let mut volume = Linear(0.5);
        volume /= Decibels(0.0);
        assert_approx_eq(volume, Linear(0.5));
        let mut volume = Decibels(0.0);
        volume /= Linear(0.501);
        assert_approx_eq(volume, Decibels(6.003246));
    }
}
