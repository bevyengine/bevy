#[allow(clippy::module_inception)]
mod shader;
mod shader_defs;
#[cfg(feature = "naga")]
mod shader_reflect_naga;
#[cfg(not(feature = "naga"))]
mod shader_reflect;

pub use shader::*;
pub use shader_defs::*;
#[cfg(feature = "naga")]
pub use shader_reflect_naga::*;
#[cfg(not(feature = "naga"))]
pub use shader_reflect::*;
