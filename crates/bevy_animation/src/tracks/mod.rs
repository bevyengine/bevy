mod fixed;
mod variable_interpolated;
mod variable_linear;

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
