use crate::{
    curves::{Curve, CurveCursor},
    interpolation::Lerp,
};

// TODO: impl Serialize, Deserialize
/// Curve with evenly spaced keyframes, in another words a curve with a fixed frame rate.
///
/// This curve maintains the faster sampling rate over a wide range of frame rates, because
/// it doesn't rely on keyframe cursor. As a downside, it will have a bigger memory foot print.
#[derive(Default, Debug)]
pub struct CurveFixed<T> {
    frame_rate: f32,
    /// Negative number of frames before the curve starts
    negative_offset: f32,
    pub keyframes: Vec<T>,
}

impl<T: Clone> Clone for CurveFixed<T> {
    fn clone(&self) -> Self {
        Self {
            frame_rate: self.frame_rate,
            negative_offset: self.negative_offset,
            keyframes: self.keyframes.clone(),
        }
    }
}

impl<T> CurveFixed<T> {
    pub fn from_keyframes(frame_rate: f32, offset: isize, keyframes: Vec<T>) -> Self {
        Self {
            frame_rate,
            negative_offset: -(offset as f32),
            keyframes,
        }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            frame_rate: 30.0,
            negative_offset: 0.0,
            keyframes: vec![v],
        }
    }

    pub fn frame_rate(&self) -> f32 {
        self.frame_rate
    }

    pub fn set_frame_rate(&mut self, frame_rate: f32) {
        self.frame_rate = frame_rate;
    }

    pub fn set_offset(&mut self, offset: isize) {
        self.negative_offset = -offset as f32;
    }

    pub fn offset(&self) -> isize {
        -self.negative_offset as isize
    }

    /// Number of keyframes
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// `true` when this `CurveFixed` doesn't have any keyframe
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.keyframes.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.keyframes.iter_mut()
    }
}

impl<T> Curve for CurveFixed<T>
where
    T: Lerp + Clone,
{
    type Output = T;

    fn duration(&self) -> f32 {
        ((self.keyframes.len() as f32 - 1.0 - self.negative_offset) / self.frame_rate).max(0.0)
    }

    fn sample(&self, time: f32) -> Self::Output {
        // Make sure to have at least one sample
        assert!(!self.keyframes.is_empty(), "track is empty");

        let t = time.mul_add(self.frame_rate, self.negative_offset);
        if t.is_sign_negative() {
            // Underflow clamp
            return self.keyframes[0].clone();
        }

        let f = t.trunc();
        let t = t - f;

        let f = f as usize;
        let f_n = self.keyframes.len() - 1;
        if f >= f_n {
            // Overflow clamp
            return self.keyframes[f_n].clone();
        }

        // Lerp the value
        T::lerp_unclamped(&self.keyframes[f], &self.keyframes[f + 1], t)
    }

    /// Same as `sample` function
    #[inline]
    fn sample_with_cursor(&self, cursor: CurveCursor, time: f32) -> (CurveCursor, Self::Output) {
        let _ = cursor;
        (0, self.sample(time))
    }
}
