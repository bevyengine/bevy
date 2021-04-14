use crate::{
    curves::Curve,
    interpolation::{Interpolate, Interpolation},
};

// TODO: Smooth interpolation are untested

// TODO: Curve/Clip need a validation during deserialization because they are
// structured as SOA (struct of arrays), so the vec's length must match

// https://github.com/niklasfrykholm/blog
// https://bitsquid.blogspot.com/search?q=animation
// http://bitsquid.blogspot.com/2009/11/bitsquid-low-level-animation-system.html
// http://archive.gamedev.net/archive/reference/articles/article1497.html (bit old)

// http://nfrechette.github.io/2016/12/07/anim_compression_key_reduction/
// https://github.com/nfrechette/acl

/// Keyframe tangents control mode
#[derive(Debug, Copy, Clone)]
pub enum TangentControl {
    Auto,
    Free,
    Broken,
    InBroken,
    OutBroken,
}

impl Default for TangentControl {
    fn default() -> Self {
        TangentControl::Auto
    }
}

// TODO: impl Serialize, Deserialize
/// Curve with sparse keyframes frames, in another words a curve with variable frame rate;
///
/// Similar in design to the [`CurveVariableLinear`](super::CurveVariableLinear) but allows
/// for smoother catmull-rom interpolations using tangents, which can further reduce the number of keyframes at
/// the cost of performance.
///
/// **NOTE** Keyframes count is limited by the [`CurveCursor`] size.
#[derive(Default, Debug)]
pub struct CurveVariable<T: Interpolate> {
    time_stamps: Vec<f32>,
    keyframes: Vec<T>,
    modes: Vec<Interpolation>,
    tangents_control: Vec<TangentControl>,
    tangents_in: Vec<T::Tangent>,
    tangents_out: Vec<T::Tangent>,
}

impl<T> Clone for CurveVariable<T>
where
    T: Interpolate + Clone,
    <T as Interpolate>::Tangent: Clone,
{
    fn clone(&self) -> Self {
        Self {
            time_stamps: self.time_stamps.clone(),
            keyframes: self.keyframes.clone(),
            modes: self.modes.clone(),
            tangents_control: self.tangents_control.clone(),
            tangents_in: self.tangents_in.clone(),
            tangents_out: self.tangents_out.clone(),
        }
    }
}

impl<T> CurveVariable<T>
where
    T: Interpolate,
{
    pub fn with_auto_tangents(samples: Vec<f32>, values: Vec<T>) -> Self {
        // TODO: Result?

        let length = samples.len();

        // Make sure both have the same length
        assert!(
            length == values.len(),
            "samples and values must have the same length"
        );

        assert!(
            values.len() <= u16::MAX as usize,
            "limit of {} keyframes exceeded",
            u16::MAX
        );

        assert!(length > 0, "empty curve");

        // Make sure the
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a < b),
            "time samples must be on ascending order"
        );

        let mut tangents = Vec::with_capacity(length);
        if length == 1 {
            tangents.push(T::FLAT_TANGENT);
        } else {
            for i in 0..length {
                let p = if i > 0 { i - 1 } else { length - 1 };
                let n = if (i + 1) < length { i + 1 } else { 0 };
                tangents.push(T::auto_tangent(
                    samples[p], samples[i], samples[n], &values[p], &values[i], &values[n],
                ));
            }
        }

        let mut tangents_control = Vec::with_capacity(length);
        tangents_control.resize(length, TangentControl::Auto);

        let mut modes = Vec::with_capacity(length);
        modes.resize(length, Interpolation::CatmullRom);

        Self {
            time_stamps: samples,
            keyframes: values,
            modes,
            tangents_control,
            tangents_in: tangents.clone(),
            tangents_out: tangents,
        }
    }

    pub fn from_line(t0: f32, t1: f32, v0: T, v1: T) -> Self {
        let mut modes = Vec::with_capacity(2);
        modes.resize(2, Interpolation::Linear);

        let mut tangents_control = Vec::with_capacity(2);
        tangents_control.resize(2, TangentControl::Auto);

        let mut tangents = Vec::with_capacity(2);
        tangents.resize(2, T::FLAT_TANGENT);

        if t0 < t1 {
            Self {
                time_stamps: vec![t0, t1],
                keyframes: vec![v0, v1],
                modes,
                tangents_control,
                tangents_in: tangents.clone(),
                tangents_out: tangents,
            }
        } else {
            Self {
                time_stamps: vec![t1, t0],
                keyframes: vec![v1, v0],
                modes,
                tangents_control,
                tangents_in: tangents.clone(),
                tangents_out: tangents,
            }
        }
    }

    pub fn from_constant(v: T) -> Self {
        Self {
            time_stamps: vec![0.0],
            keyframes: vec![v],
            modes: vec![Interpolation::CatmullRom],
            tangents_control: vec![TangentControl::Auto],
            tangents_in: vec![T::FLAT_TANGENT],
            tangents_out: vec![T::FLAT_TANGENT],
        }
    }

    // TODO: Edit methods

    /// Updates keyframes marked with `TangentControl::Auto`
    pub fn update_tangents(&mut self) {
        let length = self.keyframes.len();
        if length == 1 {
            self.tangents_in[0] = T::FLAT_TANGENT;
            self.tangents_out[0] = T::FLAT_TANGENT;
        } else {
            for i in 0..length {
                let p = if i > 0 { i - 1 } else { length - 1 };
                let n = if (i + 1) < length { i + 1 } else { 0 };

                let tangent = T::auto_tangent(
                    self.time_stamps[p],
                    self.time_stamps[i],
                    self.time_stamps[n],
                    &self.keyframes[p].clone(),
                    &self.keyframes[i].clone(),
                    &self.keyframes[n].clone(),
                );

                self.tangents_in[i] = tangent;
                self.tangents_out[i] = tangent;
            }
        }
    }

    /// Make sure the first keyframe starts at time `0.0`
    #[inline]
    pub fn remove_time_offset(&mut self) {
        self.apply_time_offset(-self.time_stamps[0]);
    }

    pub fn apply_time_offset(&mut self, time_offset: f32) {
        self.time_stamps.iter_mut().for_each(|t| *t += time_offset);
    }
}

impl<T> Curve for CurveVariable<T>
where
    T: Interpolate + Clone + 'static,
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
        // Adjust for the current keyframe cursor
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

        let a = i as usize;
        let b = cursor as usize;
        let value = T::interpolate_unclamped(
            &self.keyframes[a],
            &self.tangents_out[a],
            &self.keyframes[b],
            &self.tangents_in[b],
            self.modes[a as usize],
            t,
        );

        (cursor, value)
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn curve_evaluation() {
//         let curve = CurveVariableInterpolated::new(
//             vec![0.0, 0.25, 0.5, 0.75, 1.0],
//             vec![0.0, 0.5, 1.0, 1.5, 2.0],
//         );
//         assert_eq!(curve.sample(0.5), 1.0);

//         let mut i0 = 0;
//         let mut e0 = 0.0;
//         for v in &[0.1, 0.3, 0.7, 0.4, 0.2, 0.0, 0.4, 0.85, 1.0] {
//             let v = *v;
//             let (i1, e1) = curve.sample_indexed(i0, v);
//             assert_eq!(e1, 2.0 * v);
//             if e1 > e0 {
//                 assert!(i1 >= i0);
//             } else {
//                 assert!(i1 <= i0);
//             }
//             e0 = e1;
//             i0 = i1;
//         }
//     }

//     #[test]
//     #[should_panic]
//     fn curve_bad_length() {
//         let _ = CurveVariableInterpolated::new(vec![0.0, 0.5, 1.0], vec![0.0, 1.0]);
//     }

//     #[test]
//     #[should_panic]
//     fn curve_time_samples_not_sorted() {
//         let _ = CurveVariableInterpolated::new(vec![0.0, 1.5, 1.0], vec![0.0, 1.0, 2.0]);
//     }
// }
