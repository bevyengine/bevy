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

/// A volume level equivalent to a non-negative float.
///
/// TODO: Docs.
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
    /// The provided value must not be negative.
    ///
    /// The returned volume is clamped to the range `[0, f32::MAX]`.
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
    /// # use bevy::audio::Volume;
    /// #
    /// let volume = Volume::from_linear(0.5);
    /// assert_eq!(volume.to_linear(), 0.5);
    /// assert_eq!(volume.to_decibels(), -6.02059991328);
    ///
    /// let volume = Volume::from_linear(-1.0);
    /// assert_eq!(volume.to_linear(), 0.0); // clamped to 0.0
    /// ```
    pub const fn from_linear(v: f32) -> Self {
        debug_assert!(v >= 0.0);

        // Manually clamp the value to the range `[0, f32::MAX]` until `f32:max`
        // is stable as a const fn:
        //
        // > `core::f32::<impl f32>::max` is not yet stable as a const fn
        //
        // TODO: Use `f32::max` as a const fn when it is stable.
        Self(if v < 0.0 { 0.0 } else { v })
    }

    /// Returns the volume in linear scale.
    pub const fn to_linear(&self) -> f32 {
        self.0
    }

    /// Create a new volume from the given decibel level.
    pub fn from_decibels(v: f32) -> Self {
        Self(ops::powf(10.0f32, v / 20.0))
    }

    /// Returns the volume in decibels.
    pub fn to_decibels(&self) -> f32 {
        20.0 * ops::log10(self.0)
    }

    /// The silent or off volume level.
    pub const SILENT: Self = Volume::from_linear(0.0);
}
