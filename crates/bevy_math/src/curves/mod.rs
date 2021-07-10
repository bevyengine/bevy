use thiserror::Error;

mod fixed;
mod variable;
mod variable_linear;

pub use fixed::*;
pub use variable::*;
pub use variable_linear::*;

use crate::interpolation::Lerp;

/// Points to a keyframe inside a given curve.
///
/// When sampling curves with variable framerate like [`CurveVariable`] and [`CurveVariableLinear`]
/// is useful to keep track of a particular keyframe near the last sampling time, this keyframe index
/// is referred as cursor and speeds up sampling when the next time is close to the previous on, that
/// happens very often when playing a animation for instance.
pub type KeyframeIndex = u16;

/// Defines a curve function usually made of keyframes
pub trait Curve {
    type Output;

    /// Curve duration in seconds
    fn duration(&self) -> f32;

    /// Time offset before the first keyframe
    fn time_offset(&self) -> f32;

    /// Number of keyframes
    fn len(&self) -> usize;

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

pub trait CurveUtils {
    type Output: Lerp + Clone;

    /// Resamples the curve preserving the loop cycle.
    ///
    /// [`CurveFixed`] only supports evenly spaced keyframes, because of that the curve duration
    /// is always a multiple of the frame rate. So resampling a curve will always round up their duration
    /// but it's still possible to preserve the loop cycle, i.e. both start and end keyframes will be remain the same,
    /// which is a very desired property.
    fn resample_preserving_loop(&self, frame_rate: f32) -> CurveFixed<Self::Output>;
}

impl<C> CurveUtils for C
where
    C: Curve,
    <Self as Curve>::Output: Lerp + Clone,
{
    type Output = <Self as Curve>::Output;

    fn resample_preserving_loop(&self, frame_rate: f32) -> CurveFixed<Self::Output> {
        // get properties
        let offset = self.time_offset();
        let duration = self.duration();

        let frame_count = (duration * frame_rate).round() as usize;
        let frame_offset = (offset * frame_rate).round() as i32;

        let normalize = 1.0 / (frame_count - 1) as f32;
        let mut cursor0 = 0;
        let keyframes = (0..frame_count)
            .into_iter()
            .map(|f| {
                let time = duration * (f as f32 * normalize) + offset;
                let (cursor1, value) = self.sample_with_cursor(cursor0, time);
                cursor0 = cursor1;
                value
            })
            .collect::<Vec<_>>();

        // TODO: copy the start and end keyframes, because f32 precision might not be enough to preserve the loop
        // keyframes[0] = self.value_at(0);
        // keyframes[frame_count - 1] = self.value_at((self.len() - 1) as KeyframeIndex);

        CurveFixed::from_keyframes(frame_rate, frame_offset, keyframes)
    }
}
