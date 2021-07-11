use std::cmp::Ordering;

use crate::{
    curves::{Curve, CurveError, KeyframeIndex},
    interpolation::Lerp,
};

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// TODO: impl Serialize, Deserialize
/// Curve with sparse keyframes frames, in another words a curve with variable frame rate;
///
/// This is a very useful curve, because it can accommodate the output of a linear reduction keyframe algorithm
/// to lower the memory foot print. As a down side it requires the use of a keyframe cursor, and
/// loses performance when the curve frame rate is higher than the curve sampling frame rate;
///
/// It can't handle discontinuities, as in two keyframes with the same timestamp.
///
/// **NOTE** Keyframes count is limited by the [`KeyframeIndex`] size.
#[derive(Default, Debug, Clone)]
pub struct CurveVariableLinear<T> {
    time_stamps: Vec<f32>,
    keyframes: Vec<T>,
}

impl<T> CurveVariableLinear<T> {
    pub fn new(samples: Vec<f32>, values: Vec<T>) -> Result<Self, CurveError> {
        let length = samples.len();

        // Make sure both have the same length
        if length != values.len() {
            return Err(CurveError::MismatchedLength);
        }

        if values.len() > KeyframeIndex::MAX as usize {
            return Err(CurveError::KeyframeLimitReached(
                KeyframeIndex::MAX as usize,
            ));
        }

        // Make sure time stamps are ordered
        if !samples
            .iter()
            .zip(samples.iter().skip(1))
            .all(|(a, b)| a < b)
        {
            return Err(CurveError::NotSorted);
        }

        Ok(Self {
            time_stamps: samples,
            keyframes: values,
        })
    }

    pub fn from_line(time0: f32, time1: f32, value0: T, value1: T) -> Self {
        if time0 < time1 {
            Self {
                time_stamps: vec![time0, time1],
                keyframes: vec![value0, value1],
            }
        } else {
            Self {
                time_stamps: vec![time1, time0],
                keyframes: vec![value1, value0],
            }
        }
    }

    pub fn from_constant(value: T) -> Self {
        Self {
            time_stamps: vec![0.0],
            keyframes: vec![value],
        }
    }

    /// Inserts a new keyframe
    ///
    /// Panics if `at` is out of bounds.
    pub fn insert(&mut self, time: f32, value: T) {
        // Keyframe length is limited by the cursor size yype that is 2 bytes,
        assert!(
            self.keyframes.len() < KeyframeIndex::MAX as usize,
            "reached keyframe limit"
        );

        if let Some(index) = self.time_stamps.iter().position(|t| time < *t) {
            self.time_stamps.insert(index, time);
            self.keyframes.insert(index, value);
        } else {
            self.time_stamps.push(time);
            self.keyframes.push(value);
        }
    }

    /// Removes a keyframe at the given index
    ///
    /// # Panics
    ///
    /// Panics if `at` is out of bounds.
    pub fn remove(&mut self, at: KeyframeIndex) -> (f32, T) {
        let index = at as usize;
        (self.time_stamps.remove(index), self.keyframes.remove(index))
    }

    /// Sets the given keyframe value
    ///
    /// # Panics
    ///
    /// Panics if `at` is out of bounds.
    #[inline]
    pub fn set_value(&mut self, at: KeyframeIndex, value: T) {
        self.keyframes[at as usize] = value;
    }

    /// Moves the given keyframe to a different point in time
    ///
    /// # Panics
    ///
    /// Panics if `at` is out of bounds.
    pub fn set_time(&mut self, at: KeyframeIndex, time: f32) -> Option<KeyframeIndex> {
        let i = at as usize;

        let mut j = i;
        let last = self.time_stamps.len() - 1;
        if self.time_stamps[j] < time {
            // Forward search
            loop {
                if j == last {
                    break;
                }

                let temp = j + 1;
                if self.time_stamps[temp] > time {
                    break;
                }

                j = temp;
            }
        } else {
            // Backward search
            loop {
                if j == 0 {
                    break;
                }

                let temp = j - 1;
                if self.time_stamps[temp] < time {
                    break;
                }

                j = temp;
            }
        }

        match i.cmp(&j) {
            Ordering::Greater => {
                // Move backward
                let k = i + 1;
                self.time_stamps[j..k].rotate_right(1);
                self.keyframes[j..k].rotate_right(1);
            }
            Ordering::Less => {
                // Move forward
                let k = j + 1;
                self.time_stamps[i..k].rotate_left(1);
                self.keyframes[i..k].rotate_left(1);
            }
            Ordering::Equal => {
                // Just update the keyframe time
                self.time_stamps[i] = time;
                return None;
            }
        }

        Some(j as KeyframeIndex)
    }

    /// Gets keyframe value at the given index
    ///
    /// # Panics
    ///
    /// Panics if `at` is out of bounds.
    #[inline]
    pub fn get_value(&self, at: KeyframeIndex) -> &T {
        &self.keyframes[at as usize]
    }

    /// Gets keyframe time at the given index
    ///
    /// # Panics
    ///
    /// Panics if `at` is out of bounds.
    #[inline]
    pub fn get_time(&self, at: KeyframeIndex) -> f32 {
        self.time_stamps[at as usize]
    }

    pub fn set_time_offset(&mut self, time_offset: f32) {
        self.time_stamps.iter_mut().for_each(|t| *t += time_offset);
    }

    pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
        self.time_stamps.iter().copied().zip(self.keyframes.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (f32, &mut T)> {
        self.time_stamps
            .iter()
            .copied()
            .zip(self.keyframes.iter_mut())
    }
}

impl<T> Curve for CurveVariableLinear<T>
where
    T: Lerp + Clone + 'static,
{
    type Output = T;

    fn duration(&self) -> f32 {
        self.time_stamps.last().copied().unwrap_or(0.0)
    }

    #[inline]
    fn time_offset(&self) -> f32 {
        self.time_stamps[0]
    }

    #[inline]
    fn len(&self) -> usize {
        self.keyframes.len()
    }

    fn sample(&self, time: f32) -> Self::Output {
        // Index guessing gives a small search optimization
        let index = if time < self.duration() * 0.5 {
            0
        } else {
            self.time_stamps.len() - 1
        };

        self.sample_with_cursor(index as KeyframeIndex, time).1
    }

    fn sample_with_cursor(
        &self,
        mut cursor: KeyframeIndex,
        time: f32,
    ) -> (KeyframeIndex, Self::Output) {
        // Adjust for the current keyframe index
        let last_cursor = (self.time_stamps.len() - 1) as KeyframeIndex;

        cursor = cursor.max(0).min(last_cursor);
        if self.time_stamps[cursor as usize] < time {
            // Forward search
            loop {
                if cursor == last_cursor {
                    return (last_cursor, self.keyframes[last_cursor as usize].clone());
                }
                cursor += 1;

                if self.time_stamps[cursor as usize] >= time {
                    break;
                }
            }
        } else {
            // Backward search
            loop {
                if cursor == 0 {
                    return (0, self.keyframes[0].clone());
                }

                let i = cursor - 1;
                if self.time_stamps[i as usize] <= time {
                    break;
                }

                cursor = i;
            }
        }

        // Lerp the value
        let i = cursor - 1;
        let previous_time = self.time_stamps[i as usize];
        let t = (time - previous_time) / (self.time_stamps[cursor as usize] - previous_time);
        debug_assert!(
            (0.0..=1.0).contains(&t),
            "t = {} but should be normalized",
            t
        ); // Checks if it's required to normalize t
        let value = T::lerp_unclamped(
            &self.keyframes[i as usize],
            &self.keyframes[cursor as usize],
            t,
        );

        (cursor, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curve_evaluation() {
        let curve = CurveVariableLinear::new(
            vec![0.0, 0.25, 0.5, 0.75, 1.0],
            vec![0.0, 0.5, 1.0, 1.5, 2.0],
        )
        .unwrap();
        assert!((curve.sample(0.5) - 1.0).abs() < f32::EPSILON);

        let mut i0 = 0;
        let mut e0 = 0.0;
        for v in &[0.1, 0.3, 0.7, 0.4, 0.2, 0.0, 0.4, 0.85, 1.0] {
            let v = *v;
            let (i1, e1) = curve.sample_with_cursor(i0, v);
            assert!((e1 - (2.0 * v)).abs() < f32::EPSILON);
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
        let _ = CurveVariableLinear::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]).unwrap();
    }

    #[test]
    #[should_panic]
    fn curve_time_samples_not_sorted() {
        let _ = CurveVariableLinear::new(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]).unwrap();
    }
}
