//! Sample-interpolated curves constructed using the [`Curve`] API.

use super::cores::{EvenCore, EvenCoreError, UnevenCore, UnevenCoreError};
use super::{Curve, Interval};

use crate::StableInterpolate;
#[cfg(feature = "bevy_reflect")]
use alloc::format;
use core::any::type_name;
use core::fmt::{self, Debug};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{utility::GenericTypePathCell, Reflect, TypePath};

#[cfg(feature = "bevy_reflect")]
mod paths {
    pub(super) const THIS_MODULE: &str = "bevy_math::curve::sample_curves";
    pub(super) const THIS_CRATE: &str = "bevy_math";
}

/// A curve that is defined by explicit neighbor interpolation over a set of evenly-spaced samples.
#[derive(Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(where T: TypePath),
    reflect(from_reflect = false, type_path = false),
)]
pub struct SampleCurve<T, I> {
    pub(crate) core: EvenCore<T>,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) interpolation: I,
}

impl<T, I> Debug for SampleCurve<T, I>
where
    EvenCore<T>: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SampleCurve")
            .field("core", &self.core)
            .field("interpolation", &type_name::<I>())
            .finish()
    }
}

/// Note: This is not a fully stable implementation of `TypePath` due to usage of `type_name`
/// for function members.
#[cfg(feature = "bevy_reflect")]
impl<T, I> TypePath for SampleCurve<T, I>
where
    T: TypePath,
    I: 'static,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "{}::SampleCurve<{},{}>",
                paths::THIS_MODULE,
                T::type_path(),
                type_name::<I>()
            )
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!("SampleCurve<{},{}>", T::type_path(), type_name::<I>())
        })
    }

    fn type_ident() -> Option<&'static str> {
        Some("SampleCurve")
    }

    fn crate_name() -> Option<&'static str> {
        Some(paths::THIS_CRATE)
    }

    fn module_path() -> Option<&'static str> {
        Some(paths::THIS_MODULE)
    }
}

impl<T, I> Curve<T> for SampleCurve<T, I>
where
    T: Clone,
    I: Fn(&T, &T, f32) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        // `EvenCore::sample_with` is implicitly clamped.
        self.core.sample_with(t, &self.interpolation)
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
    }
}

impl<T, I> SampleCurve<T, I> {
    /// Create a new [`SampleCurve`] using the specified `interpolation` to interpolate between
    /// the given `samples`. An error is returned if there are not at least 2 samples or if the
    /// given `domain` is unbounded.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    pub fn new(
        domain: Interval,
        samples: impl IntoIterator<Item = T>,
        interpolation: I,
    ) -> Result<Self, EvenCoreError>
    where
        I: Fn(&T, &T, f32) -> T,
    {
        Ok(Self {
            core: EvenCore::new(domain, samples)?,
            interpolation,
        })
    }
}

/// A curve that is defined by neighbor interpolation over a set of evenly-spaced samples,
/// interpolated automatically using [a particularly well-behaved interpolation].
///
/// [a particularly well-behaved interpolation]: StableInterpolate
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct SampleAutoCurve<T> {
    pub(crate) core: EvenCore<T>,
}

impl<T> Curve<T> for SampleAutoCurve<T>
where
    T: StableInterpolate,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        // `EvenCore::sample_with` is implicitly clamped.
        self.core
            .sample_with(t, <T as StableInterpolate>::interpolate_stable)
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
    }
}

impl<T> SampleAutoCurve<T> {
    /// Create a new [`SampleCurve`] using type-inferred interpolation to interpolate between
    /// the given `samples`. An error is returned if there are not at least 2 samples or if the
    /// given `domain` is unbounded.
    pub fn new(
        domain: Interval,
        samples: impl IntoIterator<Item = T>,
    ) -> Result<Self, EvenCoreError> {
        Ok(Self {
            core: EvenCore::new(domain, samples)?,
        })
    }
}

/// A curve that is defined by interpolation over unevenly spaced samples with explicit
/// interpolation.
#[derive(Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(where T: TypePath),
    reflect(from_reflect = false, type_path = false),
)]
pub struct UnevenSampleCurve<T, I> {
    pub(crate) core: UnevenCore<T>,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) interpolation: I,
}

impl<T, I> Debug for UnevenSampleCurve<T, I>
where
    UnevenCore<T>: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SampleCurve")
            .field("core", &self.core)
            .field("interpolation", &type_name::<I>())
            .finish()
    }
}

/// Note: This is not a fully stable implementation of `TypePath` due to usage of `type_name`
/// for function members.
#[cfg(feature = "bevy_reflect")]
impl<T, I> TypePath for UnevenSampleCurve<T, I>
where
    T: TypePath,
    I: 'static,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "{}::UnevenSampleCurve<{},{}>",
                paths::THIS_MODULE,
                T::type_path(),
                type_name::<I>()
            )
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!("UnevenSampleCurve<{},{}>", T::type_path(), type_name::<I>())
        })
    }

    fn type_ident() -> Option<&'static str> {
        Some("UnevenSampleCurve")
    }

    fn crate_name() -> Option<&'static str> {
        Some(paths::THIS_CRATE)
    }

    fn module_path() -> Option<&'static str> {
        Some(paths::THIS_MODULE)
    }
}

impl<T, I> Curve<T> for UnevenSampleCurve<T, I>
where
    T: Clone,
    I: Fn(&T, &T, f32) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        // `UnevenCore::sample_with` is implicitly clamped.
        self.core.sample_with(t, &self.interpolation)
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
    }
}

impl<T, I> UnevenSampleCurve<T, I> {
    /// Create a new [`UnevenSampleCurve`] using the provided `interpolation` to interpolate
    /// between adjacent `timed_samples`. The given samples are filtered to finite times and
    /// sorted internally; if there are not at least 2 valid timed samples, an error will be
    /// returned.
    ///
    /// The interpolation takes two values by reference together with a scalar parameter and
    /// produces an owned value. The expectation is that `interpolation(&x, &y, 0.0)` and
    /// `interpolation(&x, &y, 1.0)` are equivalent to `x` and `y` respectively.
    pub fn new(
        timed_samples: impl IntoIterator<Item = (f32, T)>,
        interpolation: I,
    ) -> Result<Self, UnevenCoreError>
    where
        I: Fn(&T, &T, f32) -> T,
    {
        Ok(Self {
            core: UnevenCore::new(timed_samples)?,
            interpolation,
        })
    }

    /// This [`UnevenSampleAutoCurve`], but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`CurveExt::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    ///
    /// [`CurveExt::reparametrize`]: super::CurveExt::reparametrize
    pub fn map_sample_times(self, f: impl Fn(f32) -> f32) -> UnevenSampleCurve<T, I> {
        Self {
            core: self.core.map_sample_times(f),
            interpolation: self.interpolation,
        }
    }
}

/// A curve that is defined by interpolation over unevenly spaced samples,
/// interpolated automatically using [a particularly well-behaved interpolation].
///
/// [a particularly well-behaved interpolation]: StableInterpolate
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct UnevenSampleAutoCurve<T> {
    pub(crate) core: UnevenCore<T>,
}

impl<T> Curve<T> for UnevenSampleAutoCurve<T>
where
    T: StableInterpolate,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.core.domain()
    }

    #[inline]
    fn sample_clamped(&self, t: f32) -> T {
        // `UnevenCore::sample_with` is implicitly clamped.
        self.core
            .sample_with(t, <T as StableInterpolate>::interpolate_stable)
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.sample_clamped(t)
    }
}

impl<T> UnevenSampleAutoCurve<T> {
    /// Create a new [`UnevenSampleAutoCurve`] from a given set of timed samples.
    ///
    /// The samples are filtered to finite times and sorted internally; if there are not
    /// at least 2 valid timed samples, an error will be returned.
    pub fn new(timed_samples: impl IntoIterator<Item = (f32, T)>) -> Result<Self, UnevenCoreError> {
        Ok(Self {
            core: UnevenCore::new(timed_samples)?,
        })
    }

    /// This [`UnevenSampleAutoCurve`], but with the sample times moved by the map `f`.
    /// In principle, when `f` is monotone, this is equivalent to [`CurveExt::reparametrize`],
    /// but the function inputs to each are inverses of one another.
    ///
    /// The samples are re-sorted by time after mapping and deduplicated by output time, so
    /// the function `f` should generally be injective over the sample times of the curve.
    ///
    /// [`CurveExt::reparametrize`]: super::CurveExt::reparametrize
    pub fn map_sample_times(self, f: impl Fn(f32) -> f32) -> UnevenSampleAutoCurve<T> {
        Self {
            core: self.core.map_sample_times(f),
        }
    }
}

#[cfg(test)]
#[cfg(feature = "bevy_reflect")]
mod tests {
    //! These tests should guarantee (by even compiling) that `SampleCurve` and `UnevenSampleCurve`
    //! can be `Reflect` under reasonable circumstances where their interpolation is defined by:
    //! - function items
    //! - 'static closures
    //! - function pointers
    use super::{SampleCurve, UnevenSampleCurve};
    use crate::{curve::Interval, VectorSpace};
    use alloc::boxed::Box;
    use bevy_reflect::Reflect;

    #[test]
    fn reflect_sample_curve() {
        fn foo(x: &f32, y: &f32, t: f32) -> f32 {
            x.lerp(*y, t)
        }
        let bar = |x: &f32, y: &f32, t: f32| x.lerp(*y, t);
        let baz: fn(&f32, &f32, f32) -> f32 = bar;

        let samples = [0.0, 1.0, 2.0];

        let _: Box<dyn Reflect> = Box::new(SampleCurve::new(Interval::UNIT, samples, foo).unwrap());
        let _: Box<dyn Reflect> = Box::new(SampleCurve::new(Interval::UNIT, samples, bar).unwrap());
        let _: Box<dyn Reflect> = Box::new(SampleCurve::new(Interval::UNIT, samples, baz).unwrap());
    }

    #[test]
    fn reflect_uneven_sample_curve() {
        fn foo(x: &f32, y: &f32, t: f32) -> f32 {
            x.lerp(*y, t)
        }
        let bar = |x: &f32, y: &f32, t: f32| x.lerp(*y, t);
        let baz: fn(&f32, &f32, f32) -> f32 = bar;

        let keyframes = [(0.0, 1.0), (1.0, 0.0), (2.0, -1.0)];

        let _: Box<dyn Reflect> = Box::new(UnevenSampleCurve::new(keyframes, foo).unwrap());
        let _: Box<dyn Reflect> = Box::new(UnevenSampleCurve::new(keyframes, bar).unwrap());
        let _: Box<dyn Reflect> = Box::new(UnevenSampleCurve::new(keyframes, baz).unwrap());
    }
    #[test]
    fn test_infer_interp_arguments() {
        // it should be possible to infer the x and y arguments of the interpolation function
        // from the input samples. If that becomes impossible, this will fail to compile.
        SampleCurve::new(Interval::UNIT, [0.0, 1.0], |x, y, t| x.lerp(*y, t)).ok();
        UnevenSampleCurve::new([(0.1, 1.0), (1.0, 3.0)], |x, y, t| x.lerp(*y, t)).ok();
    }
}
