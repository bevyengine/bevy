use bevy_reflect::prelude::*;

/// Controls how a punctual light's intensity falls off over distance.
///
/// All modes are clamped to zero at the configured light range.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
#[reflect(Default, Debug, Clone, PartialEq, Hash)]
pub enum LightFalloff {
    /// Uses Bevy's existing physically-motivated inverse-square attenuation.
    #[default]
    InverseSquare,
    /// Decreases intensity linearly from the light origin to its range.
    Linear,
    /// Uses an exponential falloff curve over the light's range.
    Exponential,
}

impl LightFalloff {
    /// The number of built-in falloff modes.
    pub const VARIANT_COUNT: usize = 3;

    /// Returns the value encoded into GPU-side flag bits for this mode.
    pub const fn shader_index(self) -> u32 {
        match self {
            Self::InverseSquare => 0,
            Self::Linear => 1,
            Self::Exponential => 2,
        }
    }

    /// Returns the stable ordering bucket used for clustered light sorting.
    pub const fn bucket_index(self) -> usize {
        self.shader_index() as usize
    }
}
