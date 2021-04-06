use super::Curve;
use crate::interpolation::Lerp;

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// TODO: impl Serialize, Deserialize
/// Curve with sparse keyframes frames, in another words a curve with variable frame rate;
///
/// Very useful curve, because can accommodate the output of a linear reduction keyframe algorithm
/// to lower the memory foot print. But as a down side require the use of keyframe cursor, and
/// loses performance when the curve frame rate is higher than the curve sampling frame rate
#[derive(Default, Debug)]
pub struct CurveVariableLinear<T> {
    time_stamps: Vec<f32>,
    keyframes: Vec<T>,
}

impl<T: Clone> Clone for CurveVariableLinear<T> {
    fn clone(&self) -> Self {
        Self {
            time_stamps: self.time_stamps.clone(),
            keyframes: self.keyframes.clone(),
        }
    }
}

impl<T> CurveVariableLinear<T> {
    pub fn new(samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Result?

        // Make sure both have the same length
        assert!(
            samples.len() == values.len(),
            "samples and values must have the same length"
        );

        assert!(
            values.len() <= u16::MAX as usize,
            "limit of {} keyframes exceeded",
            u16::MAX
        );

        assert!(!samples.is_empty(), "empty curve");

        // Make sure the
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a < b),
            "time samples must be on ascending order"
        );

        Self {
            time_stamps: samples,
            keyframes: values,
        }
    }

    pub fn from_line(t0: f32, t1: f32, v0: T, v1: T) -> Self {
        Self {
            time_stamps: if t1 >= t0 { vec![t0, t1] } else { vec![t1, t0] },
            keyframes: vec![v0, v1],
        }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            time_stamps: vec![0.0],
            keyframes: vec![v],
        }
    }

    // pub fn insert(&mut self, time_sample: f32, value: T) {
    // }

    // pub fn remove(&mut self, index: usize) {
    //assert!(samples.len() > 1, "curve can't be empty");
    // }

    pub fn add_offset_time(&mut self, time_offset: f32) {
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

    fn sample(&self, time: f32) -> Self::Output {
        // Index guessing gives a small search optimization
        let index = if time < self.duration() * 0.5 {
            0
        } else {
            self.time_stamps.len() - 1
        };

        self.sample_with_cursor(index as u16, time).1
    }

    fn sample_with_cursor(&self, mut cursor: u16, time: f32) -> (u16, Self::Output) {
        // Adjust for the current keyframe index
        let last_cursor = (self.time_stamps.len() - 1) as u16;

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
        let value = T::lerp(
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
        );
        assert_eq!(curve.sample(0.5), 1.0);

        let mut i0 = 0;
        let mut e0 = 0.0;
        for v in &[0.1, 0.3, 0.7, 0.4, 0.2, 0.0, 0.4, 0.85, 1.0] {
            let v = *v;
            let (i1, e1) = curve.sample_with_cursor(i0, v);
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
        let _ = CurveVariableLinear::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]);
    }

    #[test]
    #[should_panic]
    fn curve_time_samples_not_sorted() {
        let _ = CurveVariableLinear::new(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]);
    }
}
