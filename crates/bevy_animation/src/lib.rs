//! Animation for the game engine Bevy

mod animation2d;
mod animation3d;
mod animation_impl;
mod common;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::EntityPath;

    #[doc(hidden)]
    pub use crate::{
        animation_player, AnimationClip, AnimationPlayer, AnimationPlugin, Keyframes, VariableCurve,
    };

    #[doc(hidden)]
    pub use crate::{
        animation_player2d, AnimationClip2d, AnimationPlayer2d, AnimationPlugin2d, Keyframes2d,
        VariableCurve2d,
    };
}

pub use crate::animation2d::{
    animation_player2d, Animation2d, AnimationClip2d, AnimationPlayer2d, AnimationPlugin2d,
    Keyframes2d, VariableCurve2d,
};
pub use crate::animation3d::{
    animation_player, AnimationClip, AnimationPlayer, AnimationPlugin, Keyframes, VariableCurve,
};
pub use crate::common::EntityPath;
