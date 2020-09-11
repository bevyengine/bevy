pub mod animation_spline;
pub mod plugin;
pub mod vec3_option;

pub use plugin::AnimationPlugin;

pub mod prelude {
    pub use crate::animation_spline::{AnimationSpline, AnimationSplineThree, LoopStyle};
    pub use crate::plugin::AnimationPlugin;
    pub use crate::vec3_option::Vec3Option;
    pub use splines::{Interpolate, Interpolation, Key, Spline};
}
