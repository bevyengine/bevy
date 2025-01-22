use bevy_derive::Deref;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// Use this [`Resource`] to control the global volume of all audio.
///
/// Note: changing this value will not affect already playing audio.
#[derive(Resource, Default, Clone, Copy, Reflect)]
#[reflect(Resource, Default)]
pub struct GlobalVolume {
    /// The global volume of all audio.
    pub volume: Volume,
}

impl GlobalVolume {
    /// Create a new [`GlobalVolume`] with the given volume.
    pub fn new(volume: f32) -> Self {
        Self {
            volume: Volume::new(volume),
        }
    }
}

/// A volume level equivalent to a non-negative float.
#[derive(Clone, Copy, Deref, Debug, Reflect)]
#[reflect(Debug)]
pub struct Volume(pub(crate) f32);

impl Default for Volume {
    fn default() -> Self {
        Self(1.0)
    }
}

impl Volume {
    /// Create a new volume level.
    pub fn new(volume: f32) -> Self {
        debug_assert!(volume >= 0.0);
        Self(f32::max(volume, 0.))
    }
    /// Get the value of the volume level.
    pub fn get(&self) -> f32 {
        self.0
    }

    /// Zero (silent) volume level
    pub const ZERO: Self = Volume(0.0);
}
