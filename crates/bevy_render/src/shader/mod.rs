#[allow(clippy::module_inception)]
mod shader;
mod shader_defs;
#[cfg(not(feature = "naga-reflect"))]
mod shader_reflect;
#[cfg(feature = "naga-reflect")]
mod shader_reflect_naga;

pub use shader::*;
pub use shader_defs::*;
#[cfg(not(feature = "naga-reflect"))]
pub use shader_reflect::*;
#[cfg(feature = "naga-reflect")]
pub use shader_reflect_naga::*;
