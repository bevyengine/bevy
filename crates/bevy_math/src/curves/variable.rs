use std::cmp::Ordering;

use crate::{
    curves::{Curve, CurveCursor, CurveError},
    interpolation::{Interpolate, Interpolation},
};

/// Keyframe tangents control mode
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TangentControl {
    /// Tangents are automatically calculated, based on the catmull-rom algorithm
    Auto,
    /// In tangent will be the same as the out tangent
    Free,
    /// Tangents will set to be [`Interpolate::FLAT_TANGENT`]
    Flat,
    /// In and out tangents can be set to a different values
    Broken,
}

impl Default for TangentControl {
    fn default() -> Self {
        TangentControl::Auto
    }
}

// TODO: impl Serialize, Deserialize
// TODO: How better handling of SOA? the length for instance is repeated and extra checks are need on deserialization
// ? NOTE: Using a AOS of value, mode and tangents in and out, decreases performance on random sampling by ~15%,
// ? sequential sampling remains unchanged
/// Curve with sparse keyframes frames, in another words a curve with variable frame rate;
///
/// Similar in design to the [`CurveVariableLinear`](super::CurveVariableLinear) but allows
/// for smoother catmull-rom interpolations using tangents, which can further reduce the number of keyframes at
/// the cost of performance;
///
/// It can't handle discontinuities, as in two keyframes with the same timestamp.
///
/// Interpolation is based on this [article](http://archive.gamedev.net/archive/reference/articles/article1497.html),
/// it's very similar to the implementation used by Unity, except that tangents doesn't have weighted mode;
///
/// **NOTE**: The maximum number of keyframes is limited by the capacity of [`CurveCursor`] (a `u16`)
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
    #[inline]
    pub fn with_flat_tangents(samples: Vec<f32>, values: Vec<T>) -> Result<Self, CurveError> {
        Self::with_tangents_and_mode(
            samples,
            values,
            TangentControl::Flat,
            Interpolation::Hermite,
        )
    }

    #[inline]
    pub fn with_auto_tangents(samples: Vec<f32>, values: Vec<T>) -> Result<Self, CurveError> {
        Self::with_tangents_and_mode(
            samples,
            values,
            TangentControl::Auto,
            Interpolation::Hermite,
        )
    }

    pub fn with_tangents_and_mode(
        samples: Vec<f32>,
        values: Vec<T>,
        tangent_control: TangentControl,
        mode: Interpolation,
    ) -> Result<Self, CurveError> {
        let length = samples.len();

        // Make sure both have the same length
        if length != values.len() {
            return Err(CurveError::MismatchedLength);
        }

        if values.len() > CurveCursor::MAX as usize {
            return Err(CurveError::KeyframeLimitReached(CurveCursor::MAX as usize));
        }

        // Make sure the
        if !samples
            .iter()
            .zip(samples.iter().skip(1))
            .all(|(a, b)| a < b)
        {
            return Err(CurveError::NotSorted);
        }

        let mut tangents = Vec::with_capacity(length);
        if tangent_control == TangentControl::Auto
            || tangent_control == TangentControl::Free
            || tangent_control == TangentControl::Broken
        {
            if length == 1 {
                tangents.push(T::FLAT_TANGENT);
            } else {
                for i in 0..length {
                    let p = if i > 0 { i - 1 } else { 0 };
                    let n = if (i + 1) < length { i + 1 } else { length - 1 };
                    tangents.push(T::auto_tangent(
                        samples[p], samples[i], samples[n], &values[p], &values[i], &values[n],
                    ));
                }
            }
        } else {
            tangents.resize(length, T::FLAT_TANGENT);
        }

        let mut tangents_control = Vec::with_capacity(length);
        tangents_control.resize(length, tangent_control);

        let mut modes = Vec::with_capacity(length);
        modes.resize(length, mode);

        Ok(Self {
            time_stamps: samples,
            keyframes: values,
            modes,
            tangents_control,
            tangents_in: tangents.clone(),
            tangents_out: tangents,
        })
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
            modes: vec![Interpolation::Hermite],
            tangents_control: vec![TangentControl::Auto],
            tangents_in: vec![T::FLAT_TANGENT],
            tangents_out: vec![T::FLAT_TANGENT],
        }
    }

    /// Insert a new keyframe
    ///
    /// ```rust
    /// use bevy_math::curves::{CurveVariable, TangentControl};
    ///
    /// # fn main() {
    /// let mut curve = CurveVariable::from_constant(0.0f32);
    /// curve.insert()
    ///     .set_time(1.0)
    ///     .set_value(2.0)
    ///     .set_tangent_control(TangentControl::Flat)
    ///     .done();
    ///
    /// assert_eq!(curve.len(), 2);
    /// # }
    /// ```
    pub fn insert(&mut self) -> CurveVariableKeyframeBuilder<T> {
        CurveVariableKeyframeBuilder {
            time: self
                .time_stamps
                .last()
                .copied()
                .map_or(0.0, |t| t + 0.03333),
            value: self.keyframes.last().unwrap().clone(),
            mode: *self.modes.last().unwrap(),
            tangent_control: TangentControl::Auto,
            tangent_in: T::FLAT_TANGENT,
            tangent_out: T::FLAT_TANGENT,
            curve: self,
        }
    }

    pub fn remove(&mut self, index: CurveCursor) {
        let i = index as usize;

        self.time_stamps.remove(i);
        self.keyframes.remove(i);
        self.modes.remove(i);
        self.tangents_control.remove(i);
        self.tangents_in.remove(i);
        self.tangents_out.remove(i);

        if i < self.keyframes.len() {
            self.adjust_tangents(i);
        }
        if i > 0 {
            self.adjust_tangents(i - 1);
        }
    }

    pub fn set_value(&mut self, index: CurveCursor, value: T) {
        let i = index as usize;
        self.keyframes[i] = value;
        self.adjust_tangents_with_neighbors(i);
    }

    pub fn set_time(&mut self, index: CurveCursor, time: f32) -> Option<CurveCursor> {
        let i = index as usize;

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
                self.modes[j..k].rotate_right(1);
                self.tangents_control[j..k].rotate_right(1);
                self.tangents_in[j..k].rotate_right(1);
                self.tangents_out[j..k].rotate_right(1);

                self.adjust_tangents_with_neighbors(j);
                self.adjust_tangents_with_neighbors(i);
            }
            Ordering::Less => {
                // Move forward
                let k = j + 1;
                self.time_stamps[i..k].rotate_left(1);
                self.keyframes[i..k].rotate_left(1);
                self.modes[i..k].rotate_left(1);
                self.tangents_control[i..k].rotate_left(1);
                self.tangents_in[i..k].rotate_left(1);
                self.tangents_out[i..k].rotate_left(1);

                self.adjust_tangents_with_neighbors(j);
                self.adjust_tangents_with_neighbors(i);
            }
            Ordering::Equal => {
                // Just update the keyframe time
                self.time_stamps[i] = time;
                self.adjust_tangents_with_neighbors(i);
                return None;
            }
        }

        Some(j as CurveCursor)
    }

    #[inline]
    pub fn set_interpolation(&mut self, index: CurveCursor, interpolation: Interpolation) {
        self.modes[index as usize] = interpolation;
    }

    #[inline]
    pub fn set_in_tangent(&mut self, index: CurveCursor, tangent: T::Tangent) {
        let i = index as usize;
        self.tangents_control[i] = TangentControl::Broken;
        self.tangents_in[i] = tangent;
    }

    #[inline]
    pub fn set_out_tangent(&mut self, index: CurveCursor, tangent: T::Tangent) {
        let i = index as usize;
        self.tangents_control[i] = TangentControl::Broken;
        self.tangents_out[i] = tangent;
    }

    #[inline]
    pub fn set_in_out_tangent(&mut self, index: CurveCursor, tangent: T::Tangent) {
        let i = index as usize;
        self.tangents_control[i] = TangentControl::Free;
        self.tangents_in[i] = tangent;
        self.tangents_out[i] = tangent;
    }

    pub fn set_tangent_control(&mut self, index: CurveCursor, tangent_control: TangentControl) {
        let i = index as usize;
        self.tangents_control[i] = tangent_control;
        self.adjust_tangents(i);
    }

    /// Adjust tangents for self and neighbors keyframes
    pub(crate) fn adjust_tangents_with_neighbors(&mut self, i: usize) {
        if i > 0 {
            self.adjust_tangents(i - 1);
        }

        self.adjust_tangents(i);

        if i < self.keyframes.len() - 1 {
            self.adjust_tangents(i + 1);
        }
    }

    /// Adjust tangents for a single keyframe according with their [`TangentControl`]
    fn adjust_tangents(&mut self, i: usize) {
        let length = self.keyframes.len();
        let mut tangent = T::FLAT_TANGENT;

        match self.tangents_control[i] {
            TangentControl::Auto => {
                if length > 2 {
                    let p = if i > 0 { i - 1 } else { 0 };
                    let n = if (i + 1) < length { i + 1 } else { length - 1 };

                    tangent = T::auto_tangent(
                        self.time_stamps[p],
                        self.time_stamps[i],
                        self.time_stamps[n],
                        &self.keyframes[p],
                        &self.keyframes[i],
                        &self.keyframes[n],
                    );
                }
            }
            TangentControl::Free => {
                // Copy left tangent into the right tangent
                self.tangents_out[i] = self.tangents_in[i];
                return;
            }
            TangentControl::Flat => {}
            _ => {
                // Do nothing
                return;
            }
        }

        self.tangents_in[i] = tangent;
        self.tangents_out[i] = tangent;
    }

    /// Rebuilds tangents for the entire curve based on each keyframe [`TangentControl`] mode
    pub fn rebuild_curve_tangents(&mut self) {
        for i in 0..self.len() {
            self.adjust_tangents(i);
        }
    }

    #[inline]
    pub fn get_value(&self, index: CurveCursor) -> &T {
        &self.keyframes[index as usize]
    }

    #[inline]
    pub fn get_time(&self, index: CurveCursor) -> f32 {
        self.time_stamps[index as usize]
    }

    #[inline]
    pub fn get_interpolation(&self, index: CurveCursor) -> Interpolation {
        self.modes[index as usize]
    }

    #[inline]
    pub fn get_tangent_control(&self, index: CurveCursor) -> TangentControl {
        self.tangents_control[index as usize]
    }

    #[inline]
    pub fn get_in_out_tangent(&self, index: CurveCursor) -> (T::Tangent, T::Tangent) {
        let i = index as usize;
        (self.tangents_in[i], self.tangents_out[i])
    }

    /// Number of keyframes
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// `true` when this `CurveFixed` doesn't have any keyframe
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn set_offset(&mut self, mut time_offset: f32) {
        time_offset -= self.offset(); // Removes current offset
        self.time_stamps.iter_mut().for_each(|t| *t += time_offset);
    }

    #[inline]
    pub fn offset(&self) -> f32 {
        self.time_stamps[0]
    }

    pub fn iter(&self) -> impl Iterator<Item = (f32, &T)> {
        self.time_stamps.iter().copied().zip(self.keyframes.iter())
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

    fn sample_with_cursor(
        &self,
        mut cursor: CurveCursor,
        time: f32,
    ) -> (CurveCursor, Self::Output) {
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
        let dt = self.time_stamps[cursor as usize] - previous_time;
        let t = (time - previous_time) / dt;
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
            dt,
        );

        (cursor, value)
    }
}

#[must_use = "use the `done` function to insert the keyframe"]
pub struct CurveVariableKeyframeBuilder<'a, T: Interpolate> {
    curve: &'a mut CurveVariable<T>,
    time: f32,
    value: T,
    mode: Interpolation,
    tangent_control: TangentControl,
    tangent_in: T::Tangent,
    tangent_out: T::Tangent,
}

impl<'a, T: Interpolate> CurveVariableKeyframeBuilder<'a, T> {
    #[inline]
    pub fn set_time(mut self, time: f32) -> Self {
        self.time = time;
        self
    }

    #[inline]
    pub fn set_value(mut self, value: T) -> Self {
        self.value = value;
        self
    }

    #[inline]
    pub fn set_mode(mut self, mode: Interpolation) -> Self {
        self.mode = mode;
        self
    }

    #[inline]
    pub fn set_tangent_control(mut self, tangent_control: TangentControl) -> Self {
        if tangent_control == TangentControl::Flat {
            self.tangent_in = T::FLAT_TANGENT;
            self.tangent_out = T::FLAT_TANGENT;
        }

        self.tangent_control = tangent_control;
        self
    }

    #[inline]
    pub fn set_in_tangent(mut self, tangent: T::Tangent) -> Self {
        self.tangent_control = TangentControl::Broken;
        self.tangent_in = tangent;
        self
    }

    #[inline]
    pub fn set_out_tangent(mut self, tangent: T::Tangent) -> Self {
        self.tangent_control = TangentControl::Broken;
        self.tangent_out = tangent;
        self
    }

    #[inline]
    pub fn set_in_out_tangent(&mut self, tangent: T::Tangent) {
        self.tangent_control = TangentControl::Free;
        self.tangent_in = tangent;
        self.tangent_out = tangent;
    }

    pub fn done(self) -> Result<CurveCursor, CurveError> {
        let index;

        if self.curve.len() >= (CurveCursor::MAX - 1) as usize {
            return Err(CurveError::KeyframeLimitReached(CurveCursor::MAX as usize));
        }

        if let Some(i) = self.curve.time_stamps.iter().position(|t| *t > self.time) {
            index = i;
            self.curve.time_stamps.insert(index, self.time);
            self.curve.keyframes.insert(index, self.value);
            self.curve.modes.insert(index, self.mode);
            self.curve
                .tangents_control
                .insert(index, self.tangent_control);
            self.curve.tangents_in.insert(index, self.tangent_in);
            self.curve.tangents_out.insert(index, self.tangent_out);
        } else {
            self.curve.time_stamps.push(self.time);
            self.curve.keyframes.push(self.value);
            self.curve.modes.push(self.mode);
            self.curve.tangents_control.push(self.tangent_control);
            self.curve.tangents_in.push(self.tangent_in);
            self.curve.tangents_out.push(self.tangent_out);

            index = self.curve.keyframes.len() - 1;
        }

        self.curve.adjust_tangents_with_neighbors(index);
        Ok(index as CurveCursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // TODO: Tests for creating, evaluating and editing the `CurveVariable`

    #[test]
    fn set_keyframe_value() {
        let mut curve =
            CurveVariable::with_auto_tangents(vec![0.0, 1.0, 2.0], vec![0.0, 1.0, 0.0]).unwrap();
        curve.set_value(0, 1.0);
        curve.set_value(1, 0.0);
        curve.set_value(2, 1.0);

        let ground_truth: Vec<f32> = vec![
            1.0,
            0.80658436,
            0.5144033,
            0.22222212,
            0.028806567,
            0.028806612,
            0.22222227,
            0.5144033,
            0.80658436,
            1.0,
        ];
        let samples = (0..10)
            .into_iter()
            .map(|i| curve.sample(2.0 * i as f32 / 9.0))
            .collect::<Vec<_>>();

        assert_eq!(ground_truth.len(), samples.len());
        assert!(ground_truth
            .iter()
            .zip(samples.iter())
            .all(|(a, b)| (a - b).abs() < std::f32::EPSILON));
    }

    #[test]
    fn set_keyframe_time() {
        let mut curve =
            CurveVariable::with_auto_tangents(vec![0.0, 1.0, 2.0, 3.0], vec![1.0, 0.0, 0.0, 0.0])
                .unwrap();

        // Don't change keyframe
        assert_eq!(curve.set_time(0, 0.0), None);
        assert_eq!(curve.set_time(1, 1.0), None);
        assert_eq!(curve.set_time(2, 2.0), None);
        assert_eq!(curve.set_time(3, 3.0), None);
        assert_eq!(curve.set_time(0, 0.5), None);

        // Change keyframe
        assert_eq!(curve.set_time(0, 1.5), Some(1));
        assert!((*curve.get_value(1) - 1.0).abs() < std::f32::EPSILON);

        assert_eq!(curve.set_time(1, 2.5), Some(2));
        assert!((*curve.get_value(2) - 1.0).abs() < std::f32::EPSILON);

        assert_eq!(curve.set_time(2, 0.0), Some(0));
        assert!((*curve.get_value(0) - 1.0).abs() < std::f32::EPSILON);
    }
}
