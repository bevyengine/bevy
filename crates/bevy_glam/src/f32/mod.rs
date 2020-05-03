mod funcs;
mod mat2;
mod mat3;
mod mat4;
mod quat;
#[cfg(feature = "transform-types")]
mod transform;
mod vec2;
mod vec2_mask;
mod vec3;
mod vec3_mask;
mod vec4;
mod vec4_mask;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
mod x86_utils;

pub(crate) use funcs::{scalar_acos, scalar_sin_cos};
pub use mat2::*;
pub use mat3::*;
pub use mat4::*;
pub use quat::*;
#[cfg(feature = "transform-types")]
pub use transform::*;
pub use vec2::*;
pub use vec2_mask::*;
pub use vec3::*;
pub use vec3_mask::*;
pub use vec4::*;
pub use vec4_mask::*;

#[cfg(feature = "mint")]
mod glam_mint;
#[cfg(feature = "mint")]
pub use glam_mint::*;

#[cfg(feature = "rand")]
mod glam_rand;
#[cfg(feature = "rand")]
pub use glam_rand::*;

#[cfg(feature = "serde")]
mod glam_serde;
#[cfg(feature = "serde")]
pub use glam_serde::*;

mod glam_zerocopy;
pub use glam_zerocopy::*;
