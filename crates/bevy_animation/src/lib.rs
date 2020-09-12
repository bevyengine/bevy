pub mod plugin;
pub mod spline_group;
pub mod vec3_option;

pub mod spline_groups {
    pub mod one;
    pub mod three;
    pub mod transform;
}

pub use plugin::AnimationPlugin;

pub mod prelude {
    pub use crate::plugin::AnimationPlugin;
    pub use crate::spline_group::{LoopStyle, SplineGroup};
    pub use crate::spline_groups::{
        one::AnimationSplineOne, three::AnimationSplineThree, transform::AnimationSplineTransform,
    };
    pub use crate::vec3_option::Vec3Option;
    pub use splines::{Interpolate, Interpolation, Key, Spline};
}
