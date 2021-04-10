mod conversions;
mod hsla;
mod linear_srgba;
mod srgba;

pub use hsla::*;
pub use linear_srgba::*;
pub use srgba::*;

pub type Color = LinSrgba;
