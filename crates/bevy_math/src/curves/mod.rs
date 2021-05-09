use thiserror::Error;

mod fixed;
mod variable;
mod variable_linear;

pub use fixed::*;
pub use variable::*;
pub use variable_linear::*;

pub type CurveCursor = u16;

pub trait Curve {
    type Output;

    fn duration(&self) -> f32;

    /// Easier to use sampling method that doesn't needs the keyframe cursor,
    /// but is more expensive in some types of curve, been always `O(n)`.
    ///
    /// This means sampling is more expensive to evaluate as the `time` gets bigger;
    fn sample(&self, time: f32) -> Self::Output;

    /// Samples the curve starting from some keyframe cursor, this make the common case `O(1)`
    ///
    /// **NOTE** Each keyframe is indexed by a `u16` to reduce memory usage when using the keyframe caching
    fn sample_with_cursor(&self, cursor: CurveCursor, time: f32) -> (CurveCursor, Self::Output);
}

#[derive(Error, Debug)]
pub enum CurveError {
    #[error("number of keyframes time stamps and values doesn't match")]
    MismatchedLength,
    #[error("limit of {0} keyframes exceeded")]
    KeyframeLimitReached(usize),
    #[error("keyframes aren't sorted by time")]
    NotSorted,
}
