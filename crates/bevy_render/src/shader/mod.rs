#[allow(clippy::module_inception)]

#[cfg(feature = "naga-glsl")]
mod preprocessor;
mod shader;
mod shader_defs;
#[cfg(feature = "spirv-reflect")]
mod shader_reflect;
#[cfg(feature = "naga-reflect")]
mod shader_reflect_naga;

pub use shader::*;
pub use shader_defs::*;
#[cfg(feature = "spirv-reflect")]
pub use shader_reflect::*;
#[cfg(feature = "naga-reflect")]
pub use shader_reflect_naga::*;
