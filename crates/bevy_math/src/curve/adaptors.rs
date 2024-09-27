//! Adaptors used by the Curve API for transforming and combining curves together.

use super::interval::*;
use super::Curve;

use core::any::type_name;
use core::fmt::{self, Debug};
use core::marker::PhantomData;

use bevy_reflect::utility::GenericTypePathCell;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{Reflect, TypePath};

const THIS_MODULE: &str = "bevy_math::curve::adaptors";
const THIS_CRATE: &str = "bevy_math";

// NOTE ON REFLECTION:
//
// Function members of structs pose an obstacle for reflection, because they don't implement
// reflection traits themselves. Some of these are more problematic than others; for example,
// `FromReflect` is basically hopeless for function members regardless, so function-containing
// adaptors will just never be `FromReflect` (at least until function item types implement
// Default, if that ever happens). Similarly, they do not implement `TypePath`, and as a result,
// those adaptors also need custom `TypePath` adaptors which use `type_name` instead.
//
// The sum total weirdness of the `Reflect` implementations amounts to this; those adaptors:
// - are currently never `FromReflect`;
// - have custom `TypePath` implementations which are not fully stable;
// - have custom `Debug` implementations which display the function only by type name.

/// A curve with a constant value over its domain.
///
/// This is a curve that holds an inner value and always produces a clone of that value when sampled.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ConstantCurve<T> {
    pub(crate) domain: Interval,
    pub(crate) value: T,
}

impl<T> ConstantCurve<T>
where
    T: Clone,
{
    /// Create a constant curve, which has the given `domain` and always produces the given `value`
    /// when sampled.
    pub fn new(domain: Interval, value: T) -> Self {
        Self { domain, value }
    }
}

impl<T> Curve<T> for ConstantCurve<T>
where
    T: Clone,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, _t: f32) -> T {
        self.value.clone()
    }
}

/// A curve defined by a function together with a fixed domain.
///
/// This is a curve that holds an inner function `f` which takes numbers (`f32`) as input and produces
/// output of type `T`. The value of this curve when sampled at time `t` is just `f(t)`.
#[derive(Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(where T: TypePath),
    reflect(from_reflect = false, type_path = false),
)]
pub struct FunctionCurve<T, F> {
    pub(crate) domain: Interval,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) f: F,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<T>,
}

impl<T, F> Debug for FunctionCurve<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunctionCurve")
            .field("domain", &self.domain)
            .field("f", &type_name::<F>())
            .field("_phantom", &self._phantom)
            .finish()
    }
}

#[cfg(feature = "bevy_reflect")]
impl<T, F: 'static> TypePath for FunctionCurve<T, F>
where
    T: TypePath,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "{}::FunctionCurve<{},{}>",
                THIS_MODULE,
                T::type_path(),
                type_name::<F>()
            )
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "FunctionCurve<{},{}>",
                T::short_type_path(),
                type_name::<F>()
            )
        })
    }

    fn type_ident() -> Option<&'static str> {
        Some("FunctionCurve")
    }

    fn crate_name() -> Option<&'static str> {
        Some(THIS_CRATE)
    }

    fn module_path() -> Option<&'static str> {
        Some(THIS_MODULE)
    }
}

impl<T, F> FunctionCurve<T, F>
where
    F: Fn(f32) -> T,
{
    /// Create a new curve with the given `domain` from the given `function`. When sampled, the
    /// `function` is evaluated at the sample time to compute the output.
    pub fn new(domain: Interval, function: F) -> Self {
        FunctionCurve {
            domain,
            f: function,
            _phantom: PhantomData,
        }
    }
}

impl<T, F> Curve<T> for FunctionCurve<T, F>
where
    F: Fn(f32) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        (self.f)(t)
    }
}

/// A curve whose samples are defined by mapping samples from another curve through a
/// given function. Curves of this type are produced by [`Curve::map`].
#[derive(Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(where S: TypePath, T: TypePath, C: TypePath),
    reflect(from_reflect = false, type_path = false),
)]
pub struct MapCurve<S, T, C, F> {
    pub(crate) preimage: C,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) f: F,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, F> Debug for MapCurve<S, T, C, F>
where
    C: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapCurve")
            .field("preimage", &self.preimage)
            .field("f", &type_name::<F>())
            .field("_phantom", &self._phantom)
            .finish()
    }
}

#[cfg(feature = "bevy_reflect")]
impl<S, T, C, F: 'static> TypePath for MapCurve<S, T, C, F>
where
    S: TypePath,
    T: TypePath,
    C: TypePath,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "{}::MapCurve<{},{},{},{}>",
                THIS_MODULE,
                S::type_path(),
                T::type_path(),
                C::type_path(),
                type_name::<F>()
            )
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "MapCurve<{},{},{},{}>",
                S::type_path(),
                T::type_path(),
                C::type_path(),
                type_name::<F>()
            )
        })
    }

    fn type_ident() -> Option<&'static str> {
        Some("MapCurve")
    }

    fn crate_name() -> Option<&'static str> {
        Some(THIS_CRATE)
    }

    fn module_path() -> Option<&'static str> {
        Some(THIS_MODULE)
    }
}

impl<S, T, C, F> Curve<T> for MapCurve<S, T, C, F>
where
    C: Curve<S>,
    F: Fn(S) -> T,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.preimage.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        (self.f)(self.preimage.sample_unchecked(t))
    }
}

/// A curve whose sample space is mapped onto that of some base curve's before sampling.
/// Curves of this type are produced by [`Curve::reparametrize`].
#[derive(Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(where T: TypePath, C: TypePath),
    reflect(from_reflect = false, type_path = false),
)]
pub struct ReparamCurve<T, C, F> {
    pub(crate) domain: Interval,
    pub(crate) base: C,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) f: F,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<T>,
}

impl<T, C, F> Debug for ReparamCurve<T, C, F>
where
    C: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReparamCurve")
            .field("domain", &self.domain)
            .field("base", &self.base)
            .field("f", &type_name::<F>())
            .field("_phantom", &self._phantom)
            .finish()
    }
}

#[cfg(feature = "bevy_reflect")]
impl<T, C, F: 'static> TypePath for ReparamCurve<T, C, F>
where
    T: TypePath,
    C: TypePath,
{
    fn type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "{}::ReparamCurve<{},{},{}>",
                THIS_MODULE,
                T::type_path(),
                C::type_path(),
                type_name::<F>()
            )
        })
    }

    fn short_type_path() -> &'static str {
        static CELL: GenericTypePathCell = GenericTypePathCell::new();
        CELL.get_or_insert::<Self, _>(|| {
            format!(
                "ReparamCurve<{},{},{}>",
                T::type_path(),
                C::type_path(),
                type_name::<F>()
            )
        })
    }

    fn type_ident() -> Option<&'static str> {
        Some("ReparamCurve")
    }

    fn crate_name() -> Option<&'static str> {
        Some(THIS_CRATE)
    }

    fn module_path() -> Option<&'static str> {
        Some(THIS_MODULE)
    }
}

impl<T, C, F> Curve<T> for ReparamCurve<T, C, F>
where
    C: Curve<T>,
    F: Fn(f32) -> f32,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        self.base.sample_unchecked((self.f)(t))
    }
}

/// A curve that has had its domain changed by a linear reparametrization (stretching and scaling).
/// Curves of this type are produced by [`Curve::reparametrize_linear`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(from_reflect = false)
)]
pub struct LinearReparamCurve<T, C> {
    /// Invariants: The domain of this curve must always be bounded.
    pub(crate) base: C,
    /// Invariants: This interval must always be bounded.
    pub(crate) new_domain: Interval,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<T>,
}

impl<T, C> Curve<T> for LinearReparamCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.new_domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        // The invariants imply this unwrap always succeeds.
        let f = self.new_domain.linear_map_to(self.base.domain()).unwrap();
        self.base.sample_unchecked(f(t))
    }
}

/// A curve that has been reparametrized by another curve, using that curve to transform the
/// sample times before sampling. Curves of this type are produced by [`Curve::reparametrize_by_curve`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct CurveReparamCurve<T, C, D> {
    pub(crate) base: C,
    pub(crate) reparam_curve: D,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for CurveReparamCurve<T, C, D>
where
    C: Curve<T>,
    D: Curve<f32>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.reparam_curve.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        let sample_time = self.reparam_curve.sample_unchecked(t);
        self.base.sample_unchecked(sample_time)
    }
}

/// A curve that is the graph of another curve over its parameter space. Curves of this type are
/// produced by [`Curve::graph`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct GraphCurve<T, C> {
    pub(crate) base: C,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<T>,
}

impl<T, C> Curve<(f32, T)> for GraphCurve<T, C>
where
    C: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.base.domain()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> (f32, T) {
        (t, self.base.sample_unchecked(t))
    }
}

/// A curve that combines the output data from two constituent curves into a tuple output. Curves
/// of this type are produced by [`Curve::zip`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ProductCurve<S, T, C, D> {
    pub(crate) domain: Interval,
    pub(crate) first: C,
    pub(crate) second: D,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<(S, T)>,
}

impl<S, T, C, D> Curve<(S, T)> for ProductCurve<S, T, C, D>
where
    C: Curve<S>,
    D: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        self.domain
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> (S, T) {
        (
            self.first.sample_unchecked(t),
            self.second.sample_unchecked(t),
        )
    }
}

/// The curve that results from chaining one curve with another. The second curve is
/// effectively reparametrized so that its start is at the end of the first.
///
/// For this to be well-formed, the first curve's domain must be right-finite and the second's
/// must be left-finite.
///
/// Curves of this type are produced by [`Curve::chain`].
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct ChainCurve<T, C, D> {
    pub(crate) first: C,
    pub(crate) second: D,
    #[cfg_attr(feature = "bevy_reflect", reflect(ignore))]
    pub(crate) _phantom: PhantomData<T>,
}

impl<T, C, D> Curve<T> for ChainCurve<T, C, D>
where
    C: Curve<T>,
    D: Curve<T>,
{
    #[inline]
    fn domain(&self) -> Interval {
        // This unwrap always succeeds because `first` has a valid Interval as its domain and the
        // length of `second` cannot be NAN. It's still fine if it's infinity.
        Interval::new(
            self.first.domain().start(),
            self.first.domain().end() + self.second.domain().length(),
        )
        .unwrap()
    }

    #[inline]
    fn sample_unchecked(&self, t: f32) -> T {
        if t > self.first.domain().end() {
            self.second.sample_unchecked(
                // `t - first.domain.end` computes the offset into the domain of the second.
                t - self.first.domain().end() + self.second.domain().start(),
            )
        } else {
            self.first.sample_unchecked(t)
        }
    }
}
