//! The bread and butter of the animation system tracks are the source of
//! data at given state and comes in 3 different flavours: `TrackFixed`, `TrackVariantLinear`  
//! and `TrackVariant`;
//!
//! Each track implements the `Track` trait but the animation system it self uses
//! the `TrackN` trait which implements a wide track type capable of outputting
//! multiple values for each sampling call, they can also be used for narrow track as well
//! (only a single output).

mod fixed;
mod n;
mod variable;
mod variable_linear;

pub use fixed::*;
pub use n::*;
pub use variable::*;
pub use variable_linear::*;

pub trait Track {
    type Output;

    fn duration(&self) -> f32;

    /// Easer to use sampling method that don't have time restrictions or needs
    /// the keyframe cursor, but is more expensive always `O(n)`. Which means
    /// sampling takes longer to evaluate as much as time get closer to curve duration
    /// and it get worse with more keyframes.
    fn sample(&self, time: f32) -> Self::Output;

    /// Samples the curve starting from some keyframe cursor, this make the common case `O(1)`
    ///
    /// **NOTE** Each keyframe is indexed by a `u16` to reduce memory usage when using the keyframe caching
    fn sample_with_cursor(&self, cursor: u16, time: f32) -> (u16, Self::Output);
}

pub type TrackBase<T> = Box<dyn Track<Output = T> + Send + Sync + 'static>;
