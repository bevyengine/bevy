use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_math::ops;
use bevy_reflect::prelude::*;

/// Use this [`Resource`] to control the global volume of all audio.
///
/// Note: Changing [`GlobalVolume`] does not affect already playing audio.
#[derive(Resource, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Resource, Debug, Default)]
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

/// A [`Volume`] represents audio volume levels in a linear scale, where `1.0`
/// is the "normal" or "default" volume and where `0.0` is the "silent" or "off"
/// or "muted" volume.
///
/// Values greater than `1.0` increase the volume, while values between `0.0`
/// and `1.0` decrease the volume.
///
/// To create a new [`Volume`] from a linear scale value, use
/// [`Volume::from_linear`].
///
/// To create a new [`Volume`] from decibels, use [`Volume::from_decibels`].
///
/// You can convert a [`Volume`] to a linear scale value using
/// [`Volume::to_linear`] and to decibels using [`Volume::to_decibels`].
#[derive(Clone, Copy, Deref, Debug, Reflect, PartialEq)]
#[reflect(Debug, PartialEq)]
pub struct Volume(pub(crate) f32);

impl Default for Volume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl From<f32> for Volume {
    fn from(linear_volume: f32) -> Self {
        Self::from_linear(linear_volume)
    }
}

impl PartialOrd for Volume {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.0.total_cmp(&other.0))
    }
}

impl Volume {
    /// Create a new [`Volume`] from the given volume in linear scale.
    ///
    /// The provided value must not be negative. The returned [`Volume`]'s
    /// underlying linear scale value is clamped to the range `[0, f32::MAX]`.
    ///
    /// # Note on negative values
    ///
    /// Bevy does not interpret negative values as phase inversion but simply
    /// clamps them to `0.0`.
    ///
    /// # Linear scale
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
    /// let volume = Volume::from_linear(0.5);
    /// assert_eq!(volume.to_linear(), 0.5);
    /// assert!(ops::abs(volume.to_decibels() - -6.0206) < EPSILON);
    ///
    /// let volume = Volume::from_linear(0.0);
    /// assert_eq!(volume.to_linear(), 0.0);
    /// assert_eq!(volume.to_decibels(), f32::NEG_INFINITY);
    ///
    /// let volume = Volume::from_linear(1.0);
    /// assert_eq!(volume.to_linear(), 1.0);
    /// assert!(ops::abs(volume.to_decibels() - 0.0) < EPSILON);
    /// ```
    pub const fn from_linear(v: f32) -> Self {
        // Manually clamp the value to the range `[0, f32::MAX]` until `f32:max`
        // is stable as a const fn:
        //
        // > `core::f32::<impl f32>::max` is not yet stable as a const fn
        //
        // TODO: Use `f32::max` as a const fn when it is stable.
        Self(if v <= 0.0 { 0.0 } else { v })
    }

    /// Returns the volume in linear scale.
    pub const fn to_linear(&self) -> f32 {
        self.0
    }

    /// Create a new [`Volume`] from the given volume in decibels.
    ///
    /// # Decibels
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
    /// let volume = Volume::from_decibels(-5.998);
    /// assert!(ops::abs(volume.to_linear() - 0.5) < EPSILON);
    ///
    /// let volume = Volume::from_decibels(f32::NEG_INFINITY);
    /// assert_eq!(volume.to_linear(), 0.0);
    ///
    /// let volume = Volume::from_decibels(0.0);
    /// assert_eq!(volume.to_linear(), 1.0);
    ///
    /// let volume = Volume::from_decibels(20.0);
    /// assert_eq!(volume.to_linear(), 10.0);
    /// ```
    pub fn from_decibels(v: f32) -> Self {
        Self(ops::powf(10.0f32, v / 20.0))
    }

    /// Returns the volume in decibels.
    ///
    /// If the volume is silent / off / muted, i.e. it's underlying linear scale
    /// is `0.0`, this method returns negative infinity.
    pub fn to_decibels(&self) -> f32 {
        20.0 * ops::log10(self.0)
    }

    /// The silent / off / muted volume level.
    pub const SILENT: Self = Volume(0.0);
}

#[cfg(test)]
mod tests {
    use super::Volume;

    /// Based on [Wikipedia's Decibel article].
    ///
    /// [Wikipedia's Decibel article]: https://web.archive.org/web/20230810185300/https://en.wikipedia.org/wiki/Decibel
    const DB_LINEAR_TABLE: [(f32, f32); 27] = [
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
        for (db, linear) in DB_LINEAR_TABLE {
            for volume in [Volume::from_linear(linear), Volume::from_decibels(db)] {
                let db_test = volume.to_decibels();
                let linear_test = volume.to_linear();

                let db_delta = db_test - db;
                let linear_relative_delta = (linear_test - linear) / linear;

                assert!(
                    db_delta.abs() < 1e-2,
                    "Expected ~{}dB, got {}dB (delta {})",
                    db,
                    db_test,
                    db_delta
                );
                assert!(
                    linear_relative_delta.abs() < 1e-3,
                    "Expected ~{}, got {} (relative delta {})",
                    linear,
                    linear_test,
                    linear_relative_delta
                );
            }
        }
    }

    #[test]
    fn volume_conversion_special() {
        assert!(
            Volume::from_decibels(f32::INFINITY)
                .to_linear()
                .is_infinite(),
            "Infinite decibels is equivalent to infinite linear scale"
        );
        assert!(
            Volume::from_linear(f32::INFINITY)
                .to_decibels()
                .is_infinite(),
            "Infinite linear scale is equivalent to infinite decibels"
        );

        assert!(
            Volume::from_linear(f32::NEG_INFINITY)
                .to_decibels()
                .is_infinite(),
            "Negative infinite linear scale is equivalent to infinite decibels"
        );
        assert!(
            Volume::from_decibels(f32::NEG_INFINITY).to_linear().abs() == 0.0,
            "Negative infinity decibels is equivalent to zero linear scale"
        );

        assert!(
            Volume::from_linear(0.0).to_decibels().is_infinite(),
            "Zero linear scale is equivalent to negative infinity decibels"
        );
        assert!(
            Volume::from_linear(-0.0).to_decibels().is_infinite(),
            "Negative zero linear scale is equivalent to negative infinity decibels"
        );

        assert!(
            Volume::from_decibels(f32::NAN).to_linear().is_nan(),
            "NaN decibels is equivalent to NaN linear scale"
        );
        assert!(
            Volume::from_linear(f32::NAN).to_decibels().is_nan(),
            "NaN linear scale is equivalent to NaN decibels"
        );
    }
}
