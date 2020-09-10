pub mod animatable;
pub mod animator;
pub mod plugin;

pub use plugin::AnimationPlugin;

pub mod prelude {
    pub use crate::animatable::{AnimTracks, Animatable, Splines, SplinesOne, SplinesVec3};
    pub use crate::animator::{AnimationLoop, Animator};
    pub use crate::plugin::AnimationPlugin;
    pub use splines::{Interpolate, Interpolation, Key, Spline};
}
