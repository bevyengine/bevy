use crate::{curves::Curve, interpolation::Lerp};

// TODO: impl Serialize, Deserialize
/// Fixed frame rate curve, each keyframe is evenly spaced
#[derive(Default, Debug)]
pub struct CurveFixed<T> {
    // ? NOTE: Has I learned from benches casting to f32 is quite expensive
    // ? so frame rate and offset values must be stored as f32
    frame_rate: f32,
    /// Negative number of frames before the curve starts
    negative_offset: f32,
    keyframes: Vec<T>,
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

    pub fn keyframes_mut(&mut self) -> &mut [T] {
        &mut self.keyframes[..]
    }

    // pub fn insert(&mut self, time_sample: f32, value: T) {
    // }

    // pub fn remove(&mut self, index: usize) {
    //assert!(samples.len() > 1, "curve can't be empty");
    // }

    /// Number of keyframes
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// True when doesn't have any keyframe
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn frame_rate(&self) -> usize {
        self.frame_rate as usize
    }

    pub fn offset(&self) -> isize {
        (-self.negative_offset) as isize
    }

    // pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
    //     self.samples.iter().copied().zip(self.keyframes.iter())
    // }

    // pub fn iter_mut(&mut self) -> impl Iterator<Item = (f32, &mut T)> {
    //     self.samples.iter().copied().zip(self.keyframes.iter_mut())
    // }
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
        T::lerp(&self.keyframes[f], &self.keyframes[f + 1], t)
    }

    /// Same as `sample` function
    #[inline]
    fn sample_with_cursor(&self, cursor: u16, time: f32) -> (u16, Self::Output) {
        let _ = cursor;
        (0, self.sample(time))
    }
}
