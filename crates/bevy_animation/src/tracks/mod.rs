// use std::any::Any;

mod fixed;
mod variable;
mod variable_linear;

pub use fixed::*;
pub use variable::*;
pub use variable_linear::*;

//use crate::interpolate::Interpolate;

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

// struct TrackUntypedRaw<T>(Box<dyn Track<Output = T> + Send + Sync + 'static>);

// pub struct TrackUntyped {
//     untyped: Box<dyn Any + Send + Sync + 'static>,
// }

// impl TrackUntyped {
//     pub fn new<T>(track: T) -> Self
//     where
//         T: Track + Send + Sync + 'static,
//         <T as Track>::Output: Send + Sync + 'static,
//     {
//         TrackUntyped {
//             // duration: curves.calculate_duration(),
//             // meta: CurveMeta::of::<T>(),
//             untyped: Box::new(TrackUntypedRaw(Box::new(track))),
//         }
//     }
// }

// pub enum TrackEnum<T: Interpolate> {
//     Fixed([u16; 1], TrackFixed<T>),
//     Wide4([u16; 4], TrackFixed<T>),
//     Wide8([u16; 8], TrackFixed<T>),
//     VarLinear([u16; 1], TrackVariableLinear<T>),
//     Var([u16; 1], TrackVariable<T>),
// }
