use std::any::TypeId;

use super::lerping::Lerp;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// TODO: impl Serialize, Deserialize
#[derive(Default, Debug)]
pub struct Curve<T> {
    // TODO: Step / Linear / Spline variants
    samples: Vec<f32>,
    //tangents: Vec<(f32, f32)>,
    values: Vec<T>,
}

impl<T: Clone> Clone for Curve<T> {
    fn clone(&self) -> Self {
        Self {
            samples: self.samples.clone(),
            values: self.values.clone(),
        }
    }
}

impl<T> Curve<T> {
    pub fn new(samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Result?

        // Make sure both have the same length
        assert!(
            samples.len() == values.len(),
            "samples and values must have the same length"
        );

        assert!(values.len() <= u16::MAX as usize, "too many keyframes");

        assert!(samples.len() > 0, "empty curve");

        // Make sure the
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a < b),
            "time samples must be on ascending order"
        );
        Self { samples, values }
    }

    pub fn from_linear(t0: f32, t1: f32, v0: T, v1: T) -> Self {
        Self {
            samples: if t1 >= t0 { vec![t0, t1] } else { vec![t1, t0] },
            values: vec![v0, v1],
        }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            samples: vec![0.0],
            values: vec![v],
        }
    }

    // pub fn insert(&mut self, time_sample: f32, value: T) {
    // }

    // pub fn remove(&mut self, index: usize) {
    //assert!(samples.len() > 1, "curve can't be empty");
    // }

    pub fn duration(&self) -> f32 {
        self.samples.last().copied().unwrap_or(0.0)
    }

    pub fn add_offset_time(&mut self, time_offset: f32) {
        self.samples.iter_mut().for_each(|t| *t += time_offset);
    }

    pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
        self.samples.iter().copied().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (f32, &mut T)> {
        self.samples.iter().copied().zip(self.values.iter_mut())
    }
}

impl<T> Curve<T>
where
    T: Lerp + Clone + 'static,
{
    // TODO: Profile sample_indexed vs sample, I want to know when use one over the other?

    /// Easer to use sampling method that don't have time restrictions or needs
    /// the keyframe index, but is more expensive always `O(n)`. Which means
    /// sampling takes longer to evaluate as much as time get closer to curve duration
    /// and it get worse with more keyframes.
    pub fn sample(&self, time: f32) -> T {
        // Index guessing gives a small search optimization
        let index = if time < self.duration() * 0.5 {
            0
        } else {
            self.samples.len() - 1
        };

        self.sample_indexed(index as u16, time).1
    }

    /// Samples the curve starting from some keyframe index, this make the common case `O(1)`
    ///
    /// **NOTE** Each keyframe is indexed by a `u16` to reduce memory usage when using the keyframe caching
    pub fn sample_indexed(&self, mut index: u16, time: f32) -> (u16, T) {
        // Adjust for the current keyframe index
        let last_index = (self.samples.len() - 1) as u16;

        index = index.max(0).min(last_index);
        if self.samples[index as usize] < time {
            // Forward search
            loop {
                if index == last_index {
                    return (last_index, self.values.last().unwrap().clone());
                }
                index += 1;

                if self.samples[index as usize] >= time {
                    break;
                }
            }
        } else {
            // Backward search
            loop {
                if index == 0 {
                    return (0, self.values.last().unwrap().clone());
                }

                let i = index - 1;
                if self.samples[i as usize] <= time {
                    break;
                }

                index = i;
            }
        }

        // Lerp the value
        let i = index - 1;
        let previous_time = self.samples[i as usize];
        let t = (time - previous_time) / (self.samples[index as usize] - previous_time);
        debug_assert!(t >= 0.0 && t <= 1.0, "t = {} but should be normalized", t); // Checks if it's required to normalize t
        let value = T::lerp(&self.values[i as usize], &self.values[index as usize], t);

        (index, value)
    }

    #[inline(always)]
    pub fn value_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    #[inline(always)]
    pub fn value_size(&self) -> usize {
        std::mem::size_of::<T>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_evaluation() {
        let curve = Curve::new(
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![0.0, 0.5, 1.0, 1.5, 2.0],
        );
        assert_eq!(curve.sample(0.5), 1.0);

        let mut i0 = 0;
        let mut e0 = 0.0;
        for v in &[0.1, 0.3, 0.7, 0.4, 0.2, 0.0, 0.4, 0.85, 1.0] {
            let v = *v;
            let (i1, e1) = curve.sample_indexed(i0, v);
            assert_eq!(e1, 2.0 * v);
            if e1 > e0 {
                assert!(i1 >= i0);
            } else {
                assert!(i1 <= i0);
            }
            e0 = e1;
            i0 = i1;
        }
    }

    #[test]
    #[should_panic]
    fn curve_bad_length() {
        let _ = Curve::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]);
    }

    #[test]
    #[should_panic]
    fn curve_time_samples_not_sorted() {
        let _ = Curve::new(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]);
    }
}
