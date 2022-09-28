//! Provides types and plugins for animations.

#![warn(missing_docs)]

pub mod anim2d;
pub mod anim3d;
pub mod common;

mod anim_impl;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::anim3d::{
        AnimationClip, AnimationPlayer, AnimationPlugin, Keyframes, VariableCurve,
    };
    #[doc(hidden)]
    pub use crate::common::EntityPath;
}

#[doc(inline)]
pub use crate::anim2d::{
    AnimationClip2d, AnimationPlayer2d, AnimationPlugin2d, Keyframes2d, VariableCurve2d,
};
#[doc(inline)]
pub use crate::anim3d::{
    AnimationClip, AnimationPlayer, AnimationPlugin, Keyframes, VariableCurve,
};
#[doc(inline)]
pub use crate::common::EntityPath;
