use thiserror::Error;

mod fixed;
mod variable;
mod variable_linear;

pub use fixed::*;
pub use variable::*;
pub use variable_linear::*;

/// Points to a keyframe inside a given curve.
pub type KeyframeIndex = u16;

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
    /// ```rust,ignore
    /// let mut time = 0.0;
    /// let mut current_cursor = 0;
    /// loop {
    ///     let (next_cursor, value) = curve.sample_with_cursor(current_cursor, time);
    ///     current_cursor = next_cursor;
    ///     time += 0.01333f;
    ///     /// ...
    /// }
    /// ```
    ///
    /// **NOTE** Each keyframe is indexed by a `u16` to reduce memory usage when using the keyframe caching
    fn sample_with_cursor(&self, cursor: KeyframeIndex, time: f32)
        -> (KeyframeIndex, Self::Output);
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
